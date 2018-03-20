use ast::Expr;
use rr::Value;

/// Executes program code and holds its state.
pub struct Interpreter {
    stack: Vec<CallFrame>,
    reg_in: Vec<Expr>,
    reg_out: Vec<Value>,
    pc: usize,
}

struct CallFrame {
    /// Real args passed to the current function.
    args: Vec<Value>,

    pc: usize,
}

impl Interpreter {
    pub fn interpret(&mut self, program: Expr) {
        self.reg_in.clear();
        self.reg_in.push(program);

        self.resume();
    }

    fn resume(&mut self) {
        loop {
            match self.reg_in.pop() {
                // Sacrebleu, a function call! Surprise, surprise.
                Some(Expr::Call(call)) => {
                    self.reg_in.extend(call.args);
                },

                Some(_) => {},

                None => break,
            }
        }
    }

    /// Execute the current register values as a function call.
    fn do_call(&mut self) {}
}

