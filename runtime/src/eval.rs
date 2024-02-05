//! This module contains the core logic of the interpreter.

use crate::{
    closure::Closure,
    controlflow::{break_return, throw_cf, BreakAction, ControlFlow},
    exceptions::Exception,
    fiber::Fiber,
    foreign::ForeignFn,
    scope::Scope,
    string::RipString,
    table,
    table::Table,
    throw,
    value::Value,
};
use futures::future::try_join_all;
use gc::Gc;
use riptide_syntax::{ast::*, parse, source::*};
use std::ops::ControlFlow::Continue;

/// Compile the given source code as a closure.
pub(crate) fn compile(
    fiber: &mut Fiber,
    file: impl Into<SourceFile>,
) -> Result<Closure, Exception> {
    let file = file.into();
    let file_name = file.name().to_string();

    match parse(file) {
        Ok(block) => Ok(compile_block(fiber, block)),
        Err(e) => throw!("error parsing {}: {}", file_name, e),
    }
}

/// Compile a block into an executable closure.
fn compile_block(fiber: &mut Fiber, block: Block) -> Closure {
    // Constructing a closure is quite easy since our interpreter is based
    // around evaluating AST nodes directly within an environment. All we have to
    // do aside from persisting the AST is capture the current environment.

    Closure {
        block,
        scope: fiber.current_scope().cloned(),
        name: None,
    }
}

/// Invoke the given value as a function with the given arguments.
pub(crate) async fn invoke(
    fiber: &mut Fiber,
    value: &Value,
    args: Vec<Value>,
) -> ControlFlow<Value> {
    match value {
        Value::Block(closure) => invoke_closure(fiber, closure, args, table!(), table!(), false).await,
        Value::ForeignFn(function) => invoke_native(fiber, function, args).await,
        value => throw_cf!("cannot invoke '{:?}' as a function", value),
    }
}

/// Invoke a block with an array of arguments.
pub(crate) async fn invoke_closure(
    fiber: &mut Fiber,
    closure: &Closure,
    args: Vec<Value>,
    bindings: Table,
    cvars: Table,
    bubble_up_return: bool,
) -> ControlFlow<Value> {
    let scope = Scope {
        name: format!("<closure:{}>", closure.block.span.as_ref().unwrap()),
        bindings,
        cvars,
        parent: closure.scope.clone(),
    };

    // Before attempting to bind args to named params, first define the implicit
    // `args` variable.
    scope.set("args", args.clone());

    let mut args = args.into_iter();

    // Bind arguments to any named params.
    if let Some(named_params) = closure.block.named_params.as_ref() {
        for named_param in named_params.iter() {
            scope.set(named_param.as_bytes(), args.next().unwrap_or_default());
        }
    }

    // Bind remaining unused args to a vararg param if defined.
    if let Some(vararg_param) = closure.block.vararg_param.as_ref() {
        scope.set(vararg_param.as_bytes(), args.collect::<Value>());
    }

    // Push the scope onto the stack.
    fiber.stack.push(Gc::new(scope));

    // Pop the scope off of the stack before returning. We use a scope guard to
    // do this to ensure that the stack is popped even if the current task
    // panics or is cancelled.
    let mut fiber = scopeguard::guard(fiber, |fiber| {
        fiber.stack.pop();
    });

    let mut last_return_value = Value::Nil;

    // Evaluate each statement in order.
    for statement in closure.block.statements.clone().into_iter() {
        match evaluate_statement(*fiber, statement).await {
            Continue(return_value) => last_return_value = return_value,

            // Stop block execution and return the given value.
            ControlFlow::Break(BreakAction::Return(value)) => if bubble_up_return {
                break_return!(value);
            } else {
                return Continue(value);
            },

            // Exception thrown; our scope guard from earlier will ensure that
            // the stack is unwound.
            ControlFlow::Break(BreakAction::Throw(mut exception)) => {
                if exception.backtrace.is_empty() {
                    exception.backtrace = fiber.backtrace().cloned().collect();
                }

                return ControlFlow::Break(BreakAction::Throw(exception));
            }
        }
    }

    Continue(last_return_value)
}

/// Invoke a native function.
async fn invoke_native(
    fiber: &mut Fiber,
    function: &ForeignFn,
    args: Vec<Value>,
) -> ControlFlow<Value> {
    // Push the scope onto the stack.
    fiber.stack.push(Gc::new(Scope {
        name: String::from("<native>"),
        bindings: Default::default(),
        cvars: Default::default(),
        parent: None,
    }));

    // Pop the scope off of the stack before returning. We use a scope guard to
    // do this to ensure that the stack is popped even if the current task
    // panics or is cancelled.
    let mut fiber = scopeguard::guard(fiber, |fiber| {
        fiber.stack.pop();
    });

    result_to_control_flow(function.call(*fiber, args).await.map_err(|mut e| {
        if e.backtrace.is_empty() {
            e.backtrace = fiber.backtrace().cloned().collect();
        }
        e
    }))
}

#[async_recursion::async_recursion(?Send)]
async fn evaluate_statement(fiber: &mut Fiber, statement: Statement) -> ControlFlow<Value> {
    match statement {
        Statement::Import(statement) => {
            evaluate_import_statement(fiber, statement).await?;
            Continue(Default::default())
        }
        Statement::Return(None) => break_return!(),
        Statement::Return(Some(expr)) => {
            let value = evaluate_expr(fiber, expr).await?;
            break_return!(value)
        }
        Statement::Pipeline(pipeline) => evaluate_pipeline(fiber, pipeline).await,
        Statement::Assignment(AssignmentStatement { target, value }) => {
            match target {
                AssignmentTarget::MemberAccess(member_access) => {
                    if let Some(table) = evaluate_expr(fiber, *member_access.0).await?.as_table() {
                        table.set(member_access.1, evaluate_expr(fiber, value).await?);
                    } else {
                        throw_cf!("cannot assign to a non-table")
                    }
                }
                AssignmentTarget::Variable(variable) => {
                    let mut value = evaluate_expr(fiber, value).await?;

                    // Set the name of the closure for debug purposes, if it
                    // doesn't already have one.
                    if let Value::Block(closure) = &value {
                        if closure.name().is_none() {
                            value = Value::Block(closure.with_name(variable.clone()));
                        }
                    }

                    fiber.set(variable, value);
                }
            }

            Continue(Value::Nil)
        }
    }
}

async fn evaluate_import_statement(
    fiber: &mut Fiber,
    statement: ImportStatement,
) -> ControlFlow<()> {
    let module_contents = result_to_control_flow(fiber.load_module(statement.path.as_str()).await)?;

    match statement.clause {
        ImportClause::Items(imports) => {
            for import in imports {
                if let Some(table) = module_contents.as_table() {
                    fiber.set(import.clone(), table.get(import));
                }
            }
        }

        ImportClause::Wildcard => {
            if let Some(table) = module_contents.as_table() {
                for key in table.keys() {
                    fiber.set(key.clone(), table.get(key));
                }
            }
        }
    }

    Continue(())
}

async fn evaluate_pipeline(fiber: &mut Fiber, pipeline: Pipeline) -> ControlFlow<Value> {
    match pipeline.0.len() {
        // If there's only one call in the pipeline, we don't need to fork and
        // can just execute the function by itself.
        1 => evaluate_call(fiber, pipeline.0.into_iter().next().unwrap()).await,

        // Fork the current fiber once for each step in the pipeline, wire up
        // pipes between them for their I/O context, and then execute each call
        // in the pipeline in their respective fibers concurrently.
        count => {
            let mut futures = Vec::new();
            let mut ios = match fiber.io.try_clone().and_then(|io| io.split_n(count)) {
                Ok(io) => io.into_iter(),
                Err(e) => return ControlFlow::Break(BreakAction::Throw(e.into())),
            };

            for call in pipeline.0.iter() {
                let mut fiber = fiber.fork();
                fiber.io = ios.next().unwrap();

                futures.push(async move {
                    match evaluate_call(&mut fiber, call.clone()).await {
                        Continue(value) => Ok(value),
                        ControlFlow::Break(action) => Err(action),
                    }
                });
            }

            match try_join_all(futures).await {
                Ok(values) => Continue(Value::List(values)),
                Err(action) => ControlFlow::Break(action),
            }
        }
    }
}

#[async_recursion::async_recursion(?Send)]
async fn evaluate_call(fiber: &mut Fiber, call: Call) -> ControlFlow<Value> {
    match call {
        Call::Named { function, args } => {
            let name = function;
            let function = fiber.get(&name);
            let arg_values = evaluate_call_args(fiber, args).await?;

            if !function.is_nil() {
                invoke(fiber, &function, arg_values).await
            } else {
                result_to_control_flow(crate::io::process::command(fiber, &name, &arg_values).await)
            }
        }
        Call::Unnamed { function, args } => {
            let function = evaluate_expr(fiber, *function).await?;
            let arg_values = evaluate_call_args(fiber, args).await?;

            invoke(fiber, &function, arg_values).await
        }
    }
}

async fn evaluate_call_args(fiber: &mut Fiber, args: Vec<CallArg>) -> ControlFlow<Vec<Value>> {
    let mut arg_values = Vec::with_capacity(args.len());

    for arg in args {
        match arg {
            CallArg::Expr(expr) => arg_values.push(evaluate_expr(fiber, expr).await?),
            CallArg::Splat(expr) => {
                let splat_items = evaluate_expr(fiber, expr).await?;

                if let Some(items) = splat_items.as_list() {
                    for item in items {
                        arg_values.push(item.clone());
                    }
                } else if !splat_items.is_nil() {
                    throw_cf!(
                        "cannot expand a {} value as function arguments",
                        splat_items.type_name()
                    );
                }
            }
        }
    }

    Continue(arg_values)
}

#[async_recursion::async_recursion(?Send)]
async fn evaluate_expr(fiber: &mut Fiber, expr: Expr) -> ControlFlow<Value> {
    match expr {
        Expr::Number(number) => Continue(Value::Number(number)),
        Expr::String(string) => Continue(Value::from(string)),
        Expr::Regex(RegexLiteral(src)) => Continue(Value::Regex(src)),
        Expr::CvarReference(cvar) => evaluate_cvar(fiber, cvar).await,
        Expr::CvarScope(cvar_scope) => evaluate_cvar_scope(fiber, cvar_scope).await,
        Expr::Substitution(substitution) => evaluate_substitution(fiber, substitution).await,
        Expr::Table(literal) => evaluate_table_literal(fiber, literal).await,
        Expr::List(list) => evaluate_list_literal(fiber, list).await,
        Expr::InterpolatedString(string) => evaluate_interpolated_string(fiber, string).await,
        Expr::MemberAccess(MemberAccess(lhs, rhs)) => {
            evaluate_member_access(fiber, *lhs, rhs).await
        }
        Expr::Block(block) => evaluate_block(fiber, block),
        Expr::Subroutine(subroutine) => evaluate_subroutine(fiber, subroutine),
        Expr::Pipeline(pipeline) => evaluate_pipeline(fiber, pipeline).await,
    }
}

fn evaluate_block(fiber: &mut Fiber, block: Block) -> ControlFlow<Value> {
    Continue(compile_block(fiber, block).into())
}

fn evaluate_subroutine(fiber: &mut Fiber, subroutine: Subroutine) -> ControlFlow<Value> {
    let variable_name = RipString::from(subroutine.name.as_str());
    let closure = compile_block(fiber, subroutine.block).with_name(subroutine.name);
    let value = Value::Block(closure);

    fiber.set(variable_name, value.clone());

    Continue(value)
}

async fn evaluate_member_access(fiber: &mut Fiber, lhs: Expr, rhs: String) -> ControlFlow<Value> {
    Continue(evaluate_expr(fiber, lhs).await?.get(rhs))
}

async fn evaluate_cvar(fiber: &mut Fiber, cvar: CvarReference) -> ControlFlow<Value> {
    Continue(fiber.get_cvar(cvar.0))
}

async fn evaluate_cvar_scope(fiber: &mut Fiber, cvar_scope: CvarScope) -> ControlFlow<Value> {
    let closure = compile_block(fiber, cvar_scope.scope);

    let cvars = table! {
        cvar_scope.name.0 => evaluate_expr(fiber, *cvar_scope.value).await?,
    };

    invoke_closure(fiber, &closure, vec![], cvars, table!(), false).await
}

async fn evaluate_substitution(
    fiber: &mut Fiber,
    substitution: Substitution,
) -> ControlFlow<Value> {
    match substitution {
        Substitution::Variable(name) => Continue(fiber.get(name)),
        Substitution::Pipeline(pipeline) => evaluate_pipeline(fiber, pipeline).await,
        _ => unimplemented!(),
    }
}

async fn evaluate_table_literal(fiber: &mut Fiber, literal: TableLiteral) -> ControlFlow<Value> {
    let table = Table::default();

    for entry in literal.0 {
        let key = evaluate_expr(fiber, entry.key).await?;
        let value = evaluate_expr(fiber, entry.value).await?;

        table.set(key.to_string(), value);
    }

    Continue(Value::from(table))
}

async fn evaluate_list_literal(fiber: &mut Fiber, list: ListLiteral) -> ControlFlow<Value> {
    let mut values = Vec::new();

    for expr in list.0 {
        values.push(evaluate_expr(fiber, expr).await?);
    }

    Continue(Value::List(values))
}

async fn evaluate_interpolated_string(
    fiber: &mut Fiber,
    string: InterpolatedString,
) -> ControlFlow<Value> {
    let mut rendered = String::new();

    for part in string.0.into_iter() {
        rendered.push_str(
            match part {
                InterpolatedStringPart::String(part) => part,
                InterpolatedStringPart::Substitution(sub) => {
                    evaluate_substitution(fiber, sub).await?.to_string()
                }
            }
            .as_str(),
        );
    }

    Continue(Value::from(rendered))
}

fn result_to_control_flow<T>(result: Result<T, Exception>) -> ControlFlow<T> {
    match result {
        Ok(value) => Continue(value),
        Err(exception) => ControlFlow::Break(BreakAction::Throw(exception)),
    }
}
