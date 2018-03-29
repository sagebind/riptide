use runtime::*;
use process;
use value::*;

pub fn spawn(interpreter: &mut Runtime) -> Result<Value, Exception> {
    let pid = process::spawn(|| {
        let child_interpreter = Runtime::new();
        // child_interpreter.execute(Exp)
    }).unwrap();

    Ok(Value::Number(pid as f64))
}

pub fn command() {}

pub fn exec() {}

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

pub fn require() -> Result<Value, Exception> {
    Ok(Value::Nil)
}

#[cfg(test)]
mod tests {
    use ast::*;
    use super::*;

    #[test]
    fn test_println() {
        let mut interpreter = Runtime::new();
        interpreter.set_global("println", println as ForeignFunction);

        interpreter.evaluate(Expr::Call(Call {
            function: Box::new(Expr::String("println".into())),
            args: vec![Expr::String("hello world".into())],
        })).unwrap();
    }
}
