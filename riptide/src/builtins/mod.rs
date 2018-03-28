use interpreter::*;
use process;
use value::*;

pub fn builtin(interpreter: &mut Interpreter, args: &[Value]) -> Result<Value, Exception> {
    let name = match args.first() {
        Some(&Value::String(ref s)) => s,
        _ => return Err(Exception::from("builtin name is required")),
    };

    let args = &args[1..];

    match name.as_ref() {
        "print" => print(interpreter, args),
        "println" => println(interpreter, args),
        _ => return Err(Exception::from(format!("unknown builtin \"{}\"", name))),
    }
}

pub fn spawn(interpreter: &mut Interpreter) -> Result<Value, Exception> {
    let pid = process::spawn(|| {
        let child_interpreter = Interpreter::new();
        // child_interpreter.execute(Exp)
    }).unwrap();

    Ok(Value::Number(pid as f64))
}

pub fn command() {}

pub fn exec() {}

pub fn print(_: &mut Interpreter, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        print!("{}", arg.to_string());
    }

    Ok(Value::Nil)
}

pub fn println(_: &mut Interpreter, args: &[Value]) -> Result<Value, Exception> {
    for arg in args.iter() {
        println!("{}", arg.to_string());
    }

    Ok(Value::Nil)
}

#[cfg(test)]
mod tests {
    use ast::*;
    use super::*;

    #[test]
    fn test_println() {
        let mut interpreter = Interpreter::new();
        interpreter.set_global("println", println as ForeignFunction);

        interpreter.execute(Expr::Call(Call {
            function: Box::new(Expr::String("println".into())),
            args: vec![Expr::String("hello world".into())],
        })).unwrap();
    }
}
