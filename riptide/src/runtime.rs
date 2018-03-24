use ast::Expr;
use fd;
use std::os::unix::io::FromRawFd;
use std::sync::atomic::*;
use value::Value;
use std::collections::HashMap;

/// Single fiber of execution. Contains both the interpeter stack state for the
/// fiber as well as any contextual handles.
pub struct Fiber {
    /// Unique fiber ID.
    id: usize,

    /// Standard input stream.
    stdin: Option<fd::ReadPipe>,

    /// Standard output stream.
    stdout: Option<fd::WritePipe>,

    /// Standard error stream.
    stderr: Option<fd::WritePipe>,

    /// List of child fibers.
    children: Vec<Fiber>,

    /// List of processes owned by this fiber.
    processes: Vec<()>,

    /// Stack of instructions to be executed.
    instruction_stack: Vec<Instruction>,

    /// Stack holding results produced by function calls or evaluation.
    result_stack: Vec<Value>,

    /// Call stack.
    call_stack: Vec<CallFrame>,
}

pub enum FiberState {
    Paused,
    Running,
    Terminated,
}

impl Fiber {
    fn create(id: usize) -> Self {
        unsafe {
            Fiber {
                id: id,
                stdin: Some(fd::ReadPipe::from_raw_fd(0)),
                stdout: Some(fd::WritePipe::from_raw_fd(1)),
                stderr: Some(fd::WritePipe::from_raw_fd(2)),
                children: Vec::new(),
                processes: Vec::new(),
                instruction_stack: Vec::new(),
                result_stack: Vec::new(),
                call_stack: Vec::new(),
            }
        }
    }

    pub fn stdin(&mut self) -> Option<&mut fd::ReadPipe> {
        self.stdin.as_mut()
    }
}

// impl Clone for Fiber {
//     /// Create a new fiber, with this fiber as its parent.
//     ///
//     /// The new fiber inherits the same file descriptors as its parent.
//     ///
//     /// This is a cheap (though not free) operation.
//     fn clone(&self) -> Self {
//         Fiber {
//             id: 2,
//             stdin: self.stdin.clone(),
//             stdout: self.stdout.clone(),
//             stderr: self.stderr.clone(),
//             children: Vec::new(),
//             processes: Vec::new(),
//             call_stack: Vec::new(),
//             expr_stack: Vec::new(),
//             result_stack: Vec::new(),
//         }
//     }
// }

struct CallFrame {
    /// Arguments given to the current function call.
    args: Vec<Value>,
}

/// Executes program code and holds its state.
pub struct Runtime {
    /// The currently running fiber, if any.
    current_fiber: Option<usize>,

    /// Active fibers.
    fibers: HashMap<usize, Fiber>,
}

/// Expressions are interpreted by translating them into a sequence of instructions. Each instruction usually takes a
/// number of arguments, which are popped off of the value stack in order.
///
/// The instructions are more high-level than your average VM. It's like a poor-man's JIT!
#[derive(Debug)]
enum Instruction {
    /// Meta-instruction indicating the given AST expression should be compiled into instructions and added to the
    /// stack.
    Compile(Expr),

    /// Pop off some number of values off the stack and execute them as a function call. The size parameter determines
    /// the number of arguments to pop (in reverse order), then the function body is popped off.
    Call(usize),
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            current_fiber: None,
            fibers: HashMap::new(),
        }
    }

    pub fn execute(&mut self, program: Expr) {
        let id = {
            let fiber = self.create_fiber();
            fiber.instruction_stack.push(Instruction::Compile(program));
            fiber.id
        };

        self.current_fiber = Some(id);
        self.run();
    }

    fn get_fiber_ref(&self, id: usize) -> Option<&Fiber> {
        self.fibers.get(&id)
    }

    fn get_fiber_mut(&mut self, id: usize) -> Option<&mut Fiber> {
        self.fibers.get_mut(&id)
    }

    fn get_current_fiber_ref(&self) -> Option<&Fiber> {
        if let Some(id) = self.current_fiber.clone() {
            self.get_fiber_ref(id)
        } else {
            None
        }
    }

    fn get_current_fiber_mut(&mut self) -> Option<&mut Fiber> {
        if let Some(id) = self.current_fiber.clone() {
            self.get_fiber_mut(id)
        } else {
            None
        }
    }

    fn run(&mut self) {
        if let Some(fiber) = self.get_current_fiber_mut() {
            while let Some(instruction) = fiber.instruction_stack.pop() {
                execute_instruction(fiber, instruction);
            }
        }
    }

    fn create_fiber(&mut self) -> &mut Fiber {
        let id = self.allocate_fiber_id();
        let fiber = Fiber::create(id);
        self.fibers.insert(id, fiber);
        self.fibers.get_mut(&id).unwrap()
    }

    /// Execute the current register values as a function call.
    fn do_call(&mut self) {}

    fn kill_fiber(&mut self, id: usize) -> bool {
        self.fibers.remove(&id).is_some()
    }

    fn allocate_fiber_id(&self) -> usize {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

        loop {
            let candidate = NEXT_ID.fetch_add(1, Ordering::SeqCst);

            if !self.fibers.contains_key(&candidate) {
                return candidate;
            }
        }
    }
}

fn execute_instruction(fiber: &mut Fiber, instruction: Instruction) {
    match instruction {
        Instruction::Compile(expr) => {
            match expr {
                // String literal.
                Expr::String(string) => {
                    fiber.result_stack.push(Value::from(string));
                },

                // TODO: Handle expands
                Expr::ExpandableString(string) => {
                    fiber.result_stack.push(Value::from(string));
                },

                // Simply convert the block into a value.
                Expr::Block(block) => {
                    fiber.result_stack.push(Value::from(block));
                },

                // Prep for a function call.
                Expr::Call(call) => {
                    // Generate the proper instructions for this function call. Since instructions are popped off of
                    // a stack, they need to be generated in the reverse order we want them to be executed in.

                    // Last is to call the function.
                    fiber.instruction_stack.push(Instruction::Call(call.args.len()));

                    // Then compile args in reverse order.
                    for arg in call.args.into_iter().rev() {
                        fiber.instruction_stack.push(Instruction::Compile(arg));
                    }

                    // And first is to compile the function name.
                    fiber.instruction_stack.push(Instruction::Compile(*call.function));
                },
            }
        },

        Instruction::Call(arg_count) => {
            let stack_offset = fiber.result_stack.len() - arg_count;
            let args = fiber.result_stack.split_off(stack_offset);

            let function = fiber.result_stack.pop().unwrap();

            println!("call: {:?}({:?})", function, args);

            fiber.call_stack.push(CallFrame {
                args: args,
            });

            // TODO: Perform the call.

            fiber.call_stack.pop();

            // Fake out the return value.
            fiber.result_stack.push(Value::Nil);
        },
    }
}

#[cfg(test)]
mod tests {
    use ast::*;
    use super::*;

    #[test]
    fn basic() {
        let mut runtime = Runtime::new();

        runtime.execute(Expr::Call(Call {
            function: Box::new(Expr::String("println".into())),
            args: vec![
                Expr::String("hello".into()),
                Expr::Call(Call {
                    function: Box::new(Expr::String("lowercase".into())),
                    args: vec![
                        Expr::String("THE".into()),
                    ],
                }),
                Expr::Call(Call {
                    function: Box::new(Expr::String("uppercase".into())),
                    args: vec![
                        Expr::String("World".into()),
                    ],
                }),
            ],
        }));
    }
}
