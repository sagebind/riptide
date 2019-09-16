use rusqlite::Connection;
use std::path::Path;

/// Shell history management.
///
/// History lookup is based on the frecency algorithm. Each entry in the history
/// includes the following information:
///
/// - The command run.
/// - The timestamp the command was run.
/// - How many times the same command has been run.
///
/// [Frecency algorithm]: https://developer.mozilla.org/en-US/docs/Mozilla/Tech/Places/Frecency_algorithm
pub struct History {
    db: Connection,
}

impl History {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ()> {
        match Connection::open(path) {
            Ok(db) => {
                Ok(Self {
                    db,
                })
            }
            // TODO
            Err(_) => Err(()),
        }
    }
}
