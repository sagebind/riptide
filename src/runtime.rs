use builtins;
use lua;
use lua::ThreadStatus;


/// Coroutine-enabled Lua runtime environment.
pub struct Runtime {
    state: lua::State,
}

impl Runtime {
    pub fn new() -> Self {
        let mut state = lua::State::new();
        state.open_libs();
        state.do_string(include_str!("init.lua"));

        // Register builtins.
        builtins::load(&mut state);

        Runtime {
            state: state,
        }
    }

    /// Execute Lua code. The code is executed in an independent coroutine.
    pub fn eval(&mut self, code: &str) {
        // Spawn a new coroutine.
        let mut coroutine = self.state.new_thread();

        if coroutine.load_string(code).is_err() {
            // TODO: handle error
        }

        loop {
            match coroutine.resume(None, 0) {
                ThreadStatus::Yield => {
                    // Coroutine yielded some output, pipe it out.
                    if coroutine.get_top() > 0 {
                        if let Some(s) = coroutine.to_str(1) {
                            println!("{}", s);
                        }
                    }
                }
                ThreadStatus::Ok => {
                    // Coroutine completed, we are finished executing.
                    if coroutine.get_top() > 0 {
                        if let Some(s) = coroutine.to_str(1) {
                            println!("{}", s);
                        }
                    }

                    break;
                }
                _ => {
                    if coroutine.get_top() > 0 {
                        if let Some(s) = coroutine.to_str(1) {
                            println!("error: {}", s);
                        }
                    }

                    break;
                }
            }
        }
    }

    pub fn set_status(&mut self, status: i32) {
        self.state.get_global("lish");
        self.state.push("status");
        self.state.push(status as f64);
        self.state.set_table(-3);
        self.state.pop(1);
    }

    pub fn get_status(&mut self) -> i32 {
        self.state.get_global("lish");
        self.state.push("status");
        self.state.get_table(-2);
        self.state.to_integer(-1) as i32
    }

    /// Get the current prompt string, if set.
    pub fn get_prompt(&mut self) -> Option<String> {
        self.state.get_global("lish");
        self.state.push("prompt");

        let prompt = match self.state.get_table(-2) {
            lua::Type::String => {
                let prompt = self.state.to_str(-1).map(|s| s.to_string());
                self.state.pop(1);
                prompt
            },
            lua::Type::Function => {
                if !self.state.pcall(0, 1, 0).is_err() {
                    let prompt = self.state.to_str(-1).map(|s| s.to_string());
                    self.state.pop(2);
                    prompt
                } else {
                    self.state.pop(1);
                    None
                }
            },
            _ => None,
        };

        self.state.pop(2);

        prompt
    }
}
