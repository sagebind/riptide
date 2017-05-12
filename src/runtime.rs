use builtins;
use lua;


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

    /// Execute Lua code.
    pub fn eval(&mut self, code: &str) {
        if self.state.do_string(code).is_err() {
            self.set_status(1);
        } else {
            self.set_status(0);
        }

        if let Some(result) = self.state.to_str(-1) {
            println!("{}", result);
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
