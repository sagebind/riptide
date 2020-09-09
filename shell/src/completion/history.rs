use super::Completer;
use crate::history::History;

pub struct HistoryCompleter {
    history: History,
}

impl HistoryCompleter {
    pub fn new(history: History) -> Self {
        Self {
            history,
        }
    }
}

impl Completer for HistoryCompleter {
    fn complete(&self, prefix: &str) -> Vec<String> {
        self.history
            .frequent_commands_starting_with(prefix)
            .into_iter()
            .map(|summary| summary.command)
            .collect()
    }
}
