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
use futures::{
    future::{
        FutureExt,
        LocalBoxFuture,
        try_join_all,
    },
    join,
};
use std::rc::Rc;

/// Compile the given source code as a closure.
pub(crate) fn compile(fiber: &mut Fiber, file: impl Into<SourceFile>, scope: Option<Table>) -> Result<Closure, Exception> {
    let file = file.into();
    let file_name = file.name().to_string();

    let block = match parse(file) {
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
            ..Default::default()
        }),
    })
}

/// Invoke the given value as a function with the given arguments.
pub(crate) async fn invoke(fiber: &mut Fiber, value: &Value, args: &[Value]) -> Result<Value, Exception> {
    match value {
        Value::Block(closure) => invoke_closure(fiber, closure, args, Table::default()).await,
        Value::ForeignFn(function) => invoke_native(fiber, function, args).await,
        value => throw!("cannot invoke '{:?}' as a function", value),
    }
}

/// Invoke a block with an array of arguments.
pub(crate) async fn invoke_closure(fiber: &mut Fiber, closure: &Closure, args: &[Value], cvars: Table) -> Result<Value, Exception> {
    let scope = Scope {
        name: Some(String::from("<closure>")),
        bindings: table! {
            "args" => args.to_vec(),
        },
        cvars,
        module: closure.scope.module.clone(),
        parent: Some(closure.scope.clone()),
        ..Default::default()
    };

    if let Some(named_params) = closure.block.named_params.as_ref() {
        for (i, param_name) in named_params.iter().enumerate() {
            scope.set(param_name.as_bytes(), args.get(i).cloned().unwrap_or(Value::Nil));
        }
    }

    fiber.stack.push(Rc::new(scope));

    let mut last_return_value = Value::Nil;

    // Evaluate each statement in order.
    for statement in closure.block.statements.clone().into_iter() {
        match evaluate_pipeline(fiber, statement).await {
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
        ..Default::default()
    }));

    let result = function.call(fiber, &args).await;

    fiber.stack.pop();

    result
}

async fn evaluate_pipeline(fiber: &mut Fiber, pipeline: Pipeline) -> Result<Value, Exception> {
    // If there's only one call in the pipeline, we don't need to fork and can just execute the function by itself.
    match pipeline.0.len() {
        1 => evaluate_call(fiber, pipeline.0.into_iter().next().unwrap()).await,

        2 => {
            let io = fiber.io.try_clone()?.split()?;

            let mut left = fiber.fork();
            left.io = io.0;

            let mut right = fiber.fork();
            right.io = io.1;

            let (a, b) = join!(
                async {
                    evaluate_call(&mut left, pipeline.0[0].clone()).await
                },
                async {
                    evaluate_call(&mut right, pipeline.0[1].clone()).await
                },
            );

            // TODO: Bail from exceptions early.
            Ok(Value::List(vec![a?, b?]))
        }

        _ => {
            let mut futures = Vec::new();
            let _ = fiber.io.try_clone()?;

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
}

fn evaluate_call(fiber: &mut Fiber, call: Call) -> LocalBoxFuture<Result<Value, Exception>> {
    async move {
        let (function, args) = match call {
            Call::Named {function, args} => (fiber.get(function), args),
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
    }.boxed_local()
}

fn evaluate_block(fiber: &mut Fiber, block: Block) -> Result<Value, Exception> {
    Ok(Value::from(Closure {
        block: block,
        scope: Rc::new(Scope {
            name: Some(String::from("<closure>")),
            module: fiber.current_scope().unwrap().module.clone(),
            parent: fiber.stack.last().cloned(),
            ..Default::default()
        }),
    }))
}

async fn evaluate_member_access(fiber: &mut Fiber, lhs: Expr, rhs: String) -> Result<Value, Exception> {
    Ok(evaluate_expr(fiber, lhs).await?.get(rhs))
}

async fn evaluate_cvar(fiber: &mut Fiber, cvar: CvarReference) -> Result<Value, Exception> {
    Ok(fiber.get_cvar(cvar.0))
}

async fn evaluate_cvar_scope(fiber: &mut Fiber, cvar_scope: CvarScope) -> Result<Value, Exception> {
    let closure = Closure {
        block: cvar_scope.scope,
        scope: Rc::new(Scope {
            name: Some(String::from("<closure>")),
            module: fiber.current_scope().unwrap().module.clone(),
            parent: fiber.stack.last().cloned(),
            ..Default::default()
        }),
    };

    let cvars = table! {
        cvar_scope.name.0 => evaluate_expr(fiber, *cvar_scope.value).await?,
    };

    invoke_closure(fiber, &closure, &[], cvars).await
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
