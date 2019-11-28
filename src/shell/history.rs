//! Shell history management.
//!
//! Shell history is stored in a SQLite database in a user's home directory, and
//! is written to in real time to ensure that history is not lost. The database
//! is organized into tables for various kinds of history
//!
//! ## `command_history`
//!
//! Whenever the user runs a command, the editor buffer contents are saved to
//! this table for future recollection. The data in this table is assumed to be
//! inserted in chronological order, and so the numeric `rowid` is used for
//! ordering.
//!
//! Contains the following columns:
//!
//! - `rowid`: Auto-incrementing integer ID of the command.
//! - `command`: The full, unmodified command line string that was executed.
//!   Could contain multiple lines if they were entered.
//! - `cwd`: The current working directory when the command was run.
//! - `pid`: The PID of the shell process that added the command. This is used
//!   for segregating the history by session in some queries.
//! - `timestamp`: Timestamp of when the command was run.
//!
//! ## Directory history

use rusqlite::{params, Connection, Statement, Rows, Row};
use std::env;
use std::error::Error;
use std::mem;
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// A connection to a history database.
pub struct History {
    db: Rc<Connection>,
}

/// A single entry in the history.
#[derive(Clone)]
pub struct CommandEntry {
    command: String,
    cwd: Option<String>,
    timestamp: SystemTime,
}

/// Aggregated information about a particular command string.
#[derive(Clone)]
pub struct CommandSummary {
    command: String,
    count: u32,
}

impl History {
    /// Open a history file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    /// Create a temporary in-memory history database.
    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        let history = Self {
            db: Rc::new(connection),
        };

        match history.get_version() {
            0 => history.instrument()?,
            1 => {},
            version => return Err(format!("unknown version: {}", version).into()),
        }

        Ok(history)
    }

    fn get_version(&self) -> i64 {
        self.db.query_row("PRAGMA user_version", params![], |row| row.get(0)).unwrap()
    }

    fn instrument(&self) -> Result<()> {
        self.db.execute_batch("
            PRAGMA user_version = 1;

            CREATE TABLE command_history (
                command TEXT NOT NULL,
                cwd TEXT,
                pid INTEGER,
                timestamp INTEGER NOT NULL
            );
        ")?;

        Ok(())
    }

    pub fn command_history(&self) -> Cursor<CommandEntry> {
        let mut statement = self.db.prepare("
            SELECT command, cwd, timestamp FROM command_history
            ORDER BY timestamp DESC
        ").unwrap();
        let rows = statement.query(params![]).unwrap();

        Cursor::new(&self.db, statement, rows)
    }

    /// Query for frequent commands.
    pub fn frequent_commands(&self) -> Cursor<CommandSummary> {
        let mut statement = self.db.prepare("
            SELECT command, count(*) AS count FROM command_history
            GROUP BY command
            ORDER BY count DESC
        ").unwrap();
        let rows = statement.query(params![]).unwrap();

        Cursor::new(&self.db, statement, rows)
    }

    /// Query for frequent commands with a prefix.
    pub fn frequent_commands_starting_with(&self, prefix: impl Into<String>) -> Cursor<CommandSummary> {
        let pattern = prefix.into()
            .replace("%", "\\%")
            .replace("\\", "\\\\") + "%";

        let mut statement = self.db.prepare(r#"
            SELECT command, count(*) AS count FROM command_history
            WHERE command LIKE ? ESCAPE "\"
            GROUP BY command
            ORDER BY count DESC
        "#).unwrap();

        let rows = statement.query(params![pattern]).unwrap();

        Cursor::new(&self.db, statement, rows)
    }

    /// Record a command and add it to the history.
    pub fn add(&self, command: impl Into<String>) {
        let cwd = env::current_dir().ok()
            .and_then(|path| path.to_str()
                .map(String::from));

        let pid = process::id();

        let timestamp = UNIX_EPOCH
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;

        self.db.execute(
            "INSERT INTO command_history (command, cwd, pid, timestamp) VALUES (?, ?, ?, ?)",
            params![command.into(), cwd, pid, timestamp],
        ).unwrap();
    }
}

pub trait FromRow: Sized {
    fn from_row(row: &Row) -> std::result::Result<Self, rusqlite::Error>;
}

impl FromRow for CommandEntry {
    fn from_row(row: &Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(Self {
            command: row.get("command")?,
            cwd: row.get("cwd")?,
            timestamp: UNIX_EPOCH + Duration::from_secs(row.get::<_, i64>("timestamp")? as u64),
        })
    }
}

impl FromRow for CommandSummary {
    fn from_row(row: &Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(Self {
            command: row.get("command")?,
            count: row.get("count")?,
        })
    }
}

/// A mutable, movable cursor into a sequence of search results for command
/// history.
pub struct Cursor<T> {
    rows: Rows<'static>,
    buffer: Vec<T>,
    index: usize,
    _statement: Statement<'static>,
    _db: Rc<Connection>,
}

impl<T> Cursor<T> {
    fn new<'a>(db: &'a Rc<Connection>, statement: Statement<'a>, rows: Rows<'a>) -> Self {
        Self {
            rows: unsafe {
                mem::transmute(rows)
            },
            buffer: Vec::new(),
            index: 0,
            _statement: unsafe {
                mem::transmute(statement)
            },
            _db: db.clone(),
        }
    }
}

impl<T: Clone + FromRow> Iterator for Cursor<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if let Some(item) = self.buffer.get(self.index) {
            self.index += 1;
            Some(item.clone())
        } else {
            // Pull new items from the db.
            match self.rows.next() {
                Ok(Some(row)) => {
                    let item = T::from_row(row).unwrap();
                    self.buffer.push(item.clone());
                    self.index += 1;
                    Some(item)
                }
                Ok(None) => None,
                Err(_) => unimplemented!(),
            }
        }
    }
}

impl<T: Clone + FromRow> Cursor<T> {
    fn prev(&mut self) -> Option<T> {
        if self.index > 0 {
            self.index -= 1;
            self.buffer.get(self.index).cloned()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get() {
        let history = History::in_memory().unwrap();

        for i in 0..9 {
            history.add(format!("echo {}", i));
        }

        let mut cursor = history.command_history();

        for i in 0..9 {
            assert_eq!(cursor.next().unwrap().command, format!("echo {}", 8 - i));
        }
    }
}
