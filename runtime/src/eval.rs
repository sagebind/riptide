//! This module contains the core logic of the interpreter.

use riptide_syntax::{
    parse,
    ast::*,
    source::*,
};
use super::{
    closure::Closure,
    exceptions::Exception,
    fiber::Fiber,
    foreign::ForeignFn,
    scope::Scope,
    table::Table,
    value::Value,
};
use futures::future::try_join_all;
use std::rc::Rc;

/// Compile the given source code as a closure.
pub(crate) fn compile(fiber: &mut Fiber, file: impl Into<SourceFile>, scope: Option<Table>) -> Result<Closure, Exception> {
    let file = file.into();
    let file_name = file.name().to_string();

    match parse(file) {
        Ok(block) => compile_block(fiber, block),
        Err(e) => throw!("error parsing {}: {}", file_name, e),
    }
}

/// Compile a block into an executable closure.
fn compile_block(fiber: &mut Fiber, block: Block) -> Result<Closure, Exception> {
    // Constructing a closure is quite easy since our interpreter is based
    // around evaluating AST nodes directly within an environment. All we have to
    // do aside from persisting the AST is capture the current environment.

    Ok(Closure {
        block,
        scope: fiber.current_scope().cloned(),
    })
}

/// Invoke the given value as a function with the given arguments.
pub(crate) async fn invoke(fiber: &mut Fiber, value: &Value, args: Vec<Value>) -> Result<Value, Exception> {
    match value {
        Value::Block(closure) => invoke_closure(fiber, closure, args, table!(), table!()).await,
        Value::ForeignFn(function) => invoke_native(fiber, function, args).await,
        value => throw!("cannot invoke '{:?}' as a function", value),
    }
}

/// Invoke a block with an array of arguments.
pub(crate) async fn invoke_closure(fiber: &mut Fiber, closure: &Closure, args: Vec<Value>, bindings: Table, cvars: Table) -> Result<Value, Exception> {
    bindings.set("args", args.to_vec());

    let scope = Scope {
        name: format!("<closure:{}>", closure.block.span),
        bindings,
        cvars,
        parent: closure.scope.clone(),
        ..Default::default()
    };

    if let Some(named_params) = closure.block.named_params.as_ref() {
        for (i, param_name) in named_params.iter().enumerate() {
            scope.set(param_name.as_bytes(), args.get(i).cloned().unwrap_or(Value::Nil));
        }
    }

    // Push the scope onto the stack.
    fiber.stack.push(Rc::new(scope));

    // Pop the scope off of the stack before returning. We use a scope guard to
    // do this to ensure that the stack is popped even if the current task
    // panics or is cancelled.
    let mut fiber = scopeguard::guard(fiber, |fiber| {
        fiber.stack.pop();
    });

    let mut last_return_value = Value::Nil;

    // Evaluate each statement in order.
    for statement in closure.block.statements.clone().into_iter() {
        match evaluate_statement(&mut *fiber, statement).await {
            Ok(return_value) => last_return_value = return_value,

            // Exception thrown; our scope guard from earlier will ensure that
            // the stack is unwound.
            Err(mut exception) => {
                if exception.backtrace.is_empty() {
                    exception.backtrace = fiber.backtrace().cloned().collect();
                }

                return Err(exception);
            },
        }
    }

    Ok(last_return_value)
}

/// Invoke a native function.
async fn invoke_native(fiber: &mut Fiber, function: &ForeignFn, args: Vec<Value>) -> Result<Value, Exception> {
    // Push the scope onto the stack.
    fiber.stack.push(Rc::new(Scope {
        name: String::from("<native>"),
        ..Default::default()
    }));

    // Pop the scope off of the stack before returning. We use a scope guard to
    // do this to ensure that the stack is popped even if the current task
    // panics or is cancelled.
    let mut fiber = scopeguard::guard(fiber, |fiber| {
        fiber.stack.pop();
    });

    function.call(&mut *fiber, args).await.map_err(|mut e| {
        if e.backtrace.is_empty() {
            e.backtrace = fiber.backtrace().cloned().collect();
        }
        e
    })
}

async fn evaluate_statement(fiber: &mut Fiber, statement: Statement) -> Result<Value, Exception> {
    match statement {
        Statement::Pipeline(pipeline) => evaluate_pipeline(fiber, pipeline).await,
        Statement::Assignment(AssignmentStatement {target, value}) => {
            match target {
                AssignmentTarget::MemberAccess(member_access) => {
                    if let Some(table) = evaluate_expr(fiber, *member_access.0).await?.as_table() {
                        table.set(member_access.1, evaluate_expr(fiber, value).await?);
                    } else {
                        throw!("cannot assign to a non-table")
                    }
                }
                AssignmentTarget::Variable(variable) => {
                    let value = evaluate_expr(fiber, value).await?;
                    fiber.set(variable, value);
                }
            }

            Ok(Value::Nil)
        },
    }
}

async fn evaluate_pipeline(fiber: &mut Fiber, pipeline: Pipeline) -> Result<Value, Exception> {
    match pipeline.0.len() {
        // If there's only one call in the pipeline, we don't need to fork and
        // can just execute the function by itself.
        1 => evaluate_call(fiber, pipeline.0.into_iter().next().unwrap()).await,

        // Fork the current fiber once for each step in the pipeline, wire up
        // pipes between them for their I/O context, and then execute each call
        // in the pipeline in their respective fibers concurrently.
        count => {
            let mut futures = Vec::new();
            let mut ios = fiber.io.try_clone()?.split_n(count)?.into_iter();

            for call in pipeline.0.iter() {
                let mut fiber = fiber.fork();
                fiber.io = ios.next().unwrap();

                futures.push(async move {
                    evaluate_call(&mut fiber, call.clone()).await
                });
            }

            try_join_all(futures)
                .await
                .map(Value::List)
        }
    }
}

#[async_recursion::async_recursion(?Send)]
async fn evaluate_call(fiber: &mut Fiber, call: Call) -> Result<Value, Exception> {
    match call {
        Call::Named {function, args} => {
            let name = function;
            let function = fiber.get(&name);

            let mut arg_values = Vec::with_capacity(args.len());
            for expr in args {
                arg_values.push(evaluate_expr(fiber, expr).await?);
            }

            if !function.is_nil() {
                invoke(fiber, &function, arg_values).await
            } else {
                crate::io::process::command(fiber, &name, &arg_values).await
            }
        },
        Call::Unnamed {function, args} => {
            let function = evaluate_expr(fiber, *function).await?;

            let mut arg_values = Vec::with_capacity(args.len());
            for expr in args {
                arg_values.push(evaluate_expr(fiber, expr).await?);
            }

            invoke(fiber, &function, arg_values).await
        },
    }
}

#[async_recursion::async_recursion(?Send)]
async fn evaluate_expr(fiber: &mut Fiber, expr: Expr) -> Result<Value, Exception> {
    match expr {
        Expr::Number(number) => Ok(Value::Number(number)),
        Expr::String(string) => Ok(Value::from(string)),
        Expr::CvarReference(cvar) => evaluate_cvar(fiber, cvar).await,
        Expr::CvarScope(cvar_scope) => evaluate_cvar_scope(fiber, cvar_scope).await,
        Expr::Substitution(substitution) => evaluate_substitution(fiber, substitution).await,
        Expr::Table(literal) => evaluate_table_literal(fiber, literal).await,
        Expr::List(list) => evaluate_list_literal(fiber, list).await,
        Expr::InterpolatedString(string) => evaluate_interpolated_string(fiber, string).await,
        Expr::MemberAccess(MemberAccess(lhs, rhs)) => evaluate_member_access(fiber, *lhs, rhs).await,
        Expr::Block(block) => evaluate_block(fiber, block),
        Expr::Pipeline(pipeline) => evaluate_pipeline(fiber, pipeline).await,
    }
}

fn evaluate_block(fiber: &mut Fiber, block: Block) -> Result<Value, Exception> {
    compile_block(fiber, block).map(Value::from)
}

async fn evaluate_member_access(fiber: &mut Fiber, lhs: Expr, rhs: String) -> Result<Value, Exception> {
    Ok(evaluate_expr(fiber, lhs).await?.get(rhs))
}

async fn evaluate_cvar(fiber: &mut Fiber, cvar: CvarReference) -> Result<Value, Exception> {
    Ok(fiber.get_cvar(cvar.0))
}

async fn evaluate_cvar_scope(fiber: &mut Fiber, cvar_scope: CvarScope) -> Result<Value, Exception> {
    let closure = compile_block(fiber, cvar_scope.scope)?;

    let cvars = table! {
        cvar_scope.name.0 => evaluate_expr(fiber, *cvar_scope.value).await?,
    };

    invoke_closure(fiber, &closure, vec![], cvars, table!()).await
}

async fn evaluate_substitution(fiber: &mut Fiber, substitution: Substitution) -> Result<Value, Exception> {
    match substitution {
        Substitution::Variable(name) => Ok(fiber.get(name)),
        Substitution::Pipeline(pipeline) => evaluate_pipeline(fiber, pipeline).await,
        _ => unimplemented!(),
    }
}

async fn evaluate_table_literal(fiber: &mut Fiber, literal: TableLiteral) -> Result<Value, Exception> {
    let table = Table::default();

    for entry in literal.0 {
        let key = evaluate_expr(fiber, entry.key).await?;
        let value = evaluate_expr(fiber, entry.value).await?;

        table.set(key.to_string(), value);
    }

    Ok(Value::from(table))
}

async fn evaluate_list_literal(fiber: &mut Fiber, list: ListLiteral) -> Result<Value, Exception> {
    let mut values = Vec::new();

    for expr in list.0 {
        values.push(evaluate_expr(fiber, expr).await?);
    }

    Ok(Value::List(values))
}

async fn evaluate_interpolated_string(fiber: &mut Fiber, string: InterpolatedString) -> Result<Value, Exception> {
    let mut rendered = String::new();

    for part in string.0.into_iter() {
        rendered.push_str(match part {
            InterpolatedStringPart::String(part) => part,
            InterpolatedStringPart::Substitution(sub) => evaluate_substitution(fiber, sub).await?.to_string(),
        }.as_str());
    }

    Ok(Value::from(rendered))
}
