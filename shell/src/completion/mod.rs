pub mod history;

pub trait Completer {
    fn complete(&self, prefix: &str) -> Vec<String>;

    fn complete_one(&self, prefix: &str) -> Option<String> {
        self.complete(prefix).drain(..).next()
    }
}

pub struct Composite {
    completers: Vec<Box<dyn Completer + 'static>>,
}

impl Composite {
    pub fn new() -> Self {
        Self {
            completers: Vec::new(),
        }
    }

    pub fn add<C: Completer + 'static>(&mut self, completer: C) {
        self.completers.push(Box::new(completer));
    }
}

impl Default for Composite {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for Composite {
    fn complete(&self, prefix: &str) -> Vec<String> {
        self.completers
            .iter()
            .flat_map(|completer| completer.complete(prefix))
            .collect()
    }
}

pub struct TestCompleter;

impl Completer for TestCompleter {
    fn complete(&self, prefix: &str) -> Vec<String> {
        vec![format!("{} - test completer", prefix)]
    }
}
