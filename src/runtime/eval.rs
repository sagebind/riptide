//! This module contains the core logic of the interpreter.

use super::{
    closure::Closure,
    exceptions::Exception,
    fiber::Fiber,
    foreign::ForeignFn,
    scope::Scope,
    syntax,
    syntax::ast::*,
    syntax::source::*,
    table::Table,
    value::*,
};
use futures::{
    future::{
        FutureExt,
        LocalBoxFuture,
        try_join_all,
    },
};
use std::rc::Rc;

/// Compile the given source code as a closure.
pub(crate) fn compile(fiber: &mut Fiber, file: impl Into<SourceFile>, scope: Option<Rc<Table>>) -> Result<Closure, Exception> {
    let file = file.into();
    let file_name = file.name().to_string();

    let block = match syntax::parse(file) {
        Ok(block) => block,
        Err(e) => throw!("error parsing: {}", e),
    };

    let module_scope = fiber.get_module_by_name(&file_name);

    Ok(Closure {
        block: block,
        scope: Rc::new(Scope {
            name: Some(format!("{}:<closure>", file_name)),
            bindings: scope.unwrap_or_default(),
            module: module_scope,
            parent: None,
        }),
    })
}

/// Invoke the given value as a function with the given arguments.
pub(crate) async fn invoke(fiber: &mut Fiber, value: &Value, args: &[Value]) -> Result<Value, Exception> {
    match value {
        Value::Block(closure) => invoke_closure(fiber, closure, args).await,
        Value::ForeignFn(function) => invoke_native(fiber, function, args).await,
        value => throw!("cannot invoke '{:?}' as a function", value),
    }
}

/// Invoke a block with an array of arguments.
pub(crate) async fn invoke_closure(fiber: &mut Fiber, closure: &Closure, args: &[Value]) -> Result<Value, Exception> {
    let scope = Scope {
        name: Some(String::from("<closure>")),
        bindings: Rc::new(table! {
            "args" => args.to_vec(),
        }),
        module: closure.scope.module.clone(),
        parent: Some(closure.scope.clone()),
    };

    if let Some(named_params) = closure.block.named_params.as_ref() {
        for (i, param_name) in named_params.iter().enumerate() {
            scope.set(param_name.as_bytes(), args.get(i).cloned().unwrap_or(Value::Nil));
        }
    }

    fiber.stack.push(Rc::new(scope));

    let mut last_return_value = Value::Nil;

    // Evaluate each statement in order.
    for statement in closure.block.statements.iter() {
        match evaluate_pipeline(fiber, &statement).await {
            Ok(return_value) => last_return_value = return_value,
            Err(exception) => {
                // Exception thrown; abort and unwind stack.
                fiber.stack.pop();
                return Err(exception);
            }
        }
    }

    fiber.stack.pop();

    Ok(last_return_value)
}

/// Invoke a native function.
async fn invoke_native(fiber: &mut Fiber, function: &ForeignFn, args: &[Value]) -> Result<Value, Exception> {
    fiber.stack.push(Rc::new(Scope {
        name: Some(String::from("<native>")),
        bindings: Default::default(),
        module: Default::default(),
        parent: None,
    }));

    let result = function.call(fiber, &args).await;

    fiber.stack.pop();

    result
}

async fn evaluate_pipeline(fiber: &mut Fiber, pipeline: &Pipeline) -> Result<Value, Exception> {
    // If there's only one call in the pipeline, we don't need to fork and can just execute the function by itself.
    if pipeline.0.len() == 1 {
        evaluate_call(fiber, pipeline.0[0].clone()).await
    } else {
        let mut futures = Vec::new();

        for call in pipeline.0.iter() {
            let mut fiber = fiber.fork();
            futures.push(async move {
                evaluate_call(&mut fiber, call.clone()).await
            });
        }

        try_join_all(futures)
            .await
            .map(Value::List)
    }
}

fn evaluate_call(fiber: &mut Fiber, call: Call) -> LocalBoxFuture<Result<Value, Exception>> {
    async move {
        let (function, args) = match call {
            Call::Named {function, args} => (get_path(fiber, &function), args),
            Call::Unnamed {function, args} => (
                {
                    let mut function = evaluate_expr(fiber, *function).await?;

                    // If the function is a string, resolve binding names first before we try to eval the item as a function.
                    if let Some(value) = function.as_string().map(|name| fiber.get(name)) {
                        function = value;
                    }

                    function
                },
                args,
            ),
        };

        let mut arg_values = Vec::with_capacity(args.len());
        for expr in args {
            arg_values.push(evaluate_expr(fiber, expr).await?);
        }

        invoke(fiber, &function, &arg_values).await
    }.boxed_local()
}

fn evaluate_expr(fiber: &mut Fiber, expr: Expr) -> LocalBoxFuture<Result<Value, Exception>> {
    async move {
        match expr {
            Expr::Number(number) => Ok(Value::Number(number)),
            Expr::String(string) => Ok(Value::from(string)),
            Expr::Substitution(substitution) => evaluate_substitution(fiber, substitution).await,
            Expr::Table(literal) => evaluate_table_literal(fiber, literal).await,
            Expr::List(list) => evaluate_list_literal(fiber, list).await,
            // TODO: Handle expands
            Expr::InterpolatedString(_) => {
                log::warn!("string interpolation not yet supported");
                Ok(Value::Nil)
            },
            Expr::Block(block) => evaluate_block(fiber, block),
            Expr::Pipeline(ref pipeline) => evaluate_pipeline(fiber, pipeline).await,
        }
    }.boxed_local()
}

fn evaluate_block(fiber: &mut Fiber, block: Block) -> Result<Value, Exception> {
    Ok(Value::from(Closure {
        block: block,
        scope: Rc::new(Scope {
            name: Some(String::from("<closure>")),
            bindings: Default::default(),
            module: fiber.current_scope().unwrap().module.clone(),
            parent: fiber.stack.last().cloned(),
        }),
    }))
}

async fn evaluate_substitution(fiber: &mut Fiber, substitution: Substitution) -> Result<Value, Exception> {
    match substitution {
        Substitution::Variable(path) => Ok(get_path(fiber, &path)),
        Substitution::Pipeline(ref pipeline) => evaluate_pipeline(fiber, pipeline).await,
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

fn get_path(fiber: &Fiber, path: &VariablePath) -> Value {
    let mut result = fiber.get(&path.0[0]);

    if path.0.len() > 1 {
        for part in &path.0[1..] {
            result = result.get(part);
        }
    }

    result
}
