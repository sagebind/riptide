use process;
use runtime::*;
use runtime::value::*;

/// Spawns a new child process and executes a given block in it.
///
/// Returns the child process PID.
pub fn spawn(interpreter: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    let pid = process::spawn(|| {
        let child_interpreter = Runtime::new();
        // child_interpreter.execute(Exp)
    }).unwrap();

    Ok(Value::Number(pid as f64))
}

/// Executes a shell command in the foreground, waiting for it to complete.
///
/// Returns the process exit code.
pub fn command(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

/// Executes a shell command, replacing the current process with the new process.
///
/// Does not return.
pub fn exec(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

pub fn print(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        print!("{}", arg.to_string());
    }

    Ok(Value::Nil)
}

pub fn println(_: &mut Runtime, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        println!("{}", arg.to_string());
    }

    Ok(Value::Nil)
}

pub fn require(_: &mut Runtime, _: &[Value]) -> Result<Value, Exception> {
    unimplemented!();
}

#[cfg(test)]
mod tests {
    use ast::*;
    use super::*;

    #[test]
    fn test_println() {
        let mut runtime = Runtime::new();
        runtime.set_global("println", println as ForeignFunction);

        runtime.evaluate(Expr::Call(Call {
            function: Box::new(Expr::String("println".into())),
            args: vec![Expr::String("hello world".into())],
        })).unwrap();
    }
}
