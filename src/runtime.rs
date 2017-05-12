extern crate lua;


/// Coroutine-enabled Lua runtime environment.
pub struct Runtime {
    state: lua::State,
}

impl Runtime {
    pub fn new() -> Self {
        let mut state = lua::State::new();
        state.open_libs();
        state.do_string(include_str!("init.lua"));

        Runtime {
            state: state,
        }
    }

    /// Execute Lua code.
    pub fn eval(&mut self, code: &str) {
        self.state.do_string(code);
    }

    /// Get the current prompt string, if set.
    pub fn get_prompt(&mut self) -> Option<&str> {
        self.state.get_global("lish");
        self.state.push_string("prompt");

        match self.state.get_table(-2) {
            lua::Type::String => {
                self.state.to_str(-1)
            },
            lua::Type::Function => {
                if !self.state.pcall(0, 1, 0).is_err() {
                    self.state.to_str(-1)
                } else {
                    self.state.pop(1);
                    None
                }
            },
            _ => None,
        }
    }
}
