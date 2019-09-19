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

use rusqlite::{params, Connection, Row, Rows, Statement};
use std::env;
use std::error::Error;
use std::path::Path;
use std::process;
use std::rc::Rc;
use std::time::{Duration, UNIX_EPOCH};

type Result<T> = std::result::Result<T, Box<Error>>;

/// A connection to a history database.
pub struct History {
    db: Rc<Connection>,
}

impl History {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    fn temporary() -> Result<Self> {
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

    // SELECT command, count(*) FROM command_history
    // GROUP BY command
    // ORDER BY count(*) DESC

    pub fn command_history(&self, sort: Sort) -> Cursor {
        Cursor::new(self.db.clone(), "")
    }

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

pub struct CursorBuilder {
    db: Rc<Connection>,
}

impl CursorBuilder {
    /// Only return results belonging to the given session.
    pub fn only_in_session(self, pid: u32) -> Self {
        self
    }

    /// Return results for the given session first before any other results.
    pub fn session_first(self, pid: u32) -> Self {
        self
    }

    pub fn build(self) -> Cursor {
        unimplemented!()
    }
}

pub enum SessionFilter {
    SessionOnly(u32),
    SessionFirst(u32),
}

pub enum Sort {
    Chronological,
    Frequency,
}

/// A mutable, movable cursor into a sequence of search results for command
/// history.
pub struct Cursor {
    db: Rc<Connection>,
    pattern: String,
    last_rowid: Option<i64>,
}

impl Cursor {
    fn new(db: Rc<Connection>, prefix: impl AsRef<str>) -> Self {
        Self {
            db,
            pattern: prefix.as_ref().replace("%", "\\%") + "%",
            last_rowid: None,
        }
    }
}

impl Iterator for Cursor {
    type Item = Item;

    fn next(&mut self) -> Option<Item> {
        if let Some(last_rowid) = self.last_rowid.take() {
            match self.db.query_row(
                "
                    SELECT rowid, command FROM command_history
                    WHERE rowid < ? AND command LIKE ? ESCAPE '\\'
                    ORDER BY rowid DESC
                ",
                params![last_rowid, self.pattern],
                Item::from_row,
            ) {
                Ok(item) => {
                    self.last_rowid = Some(item.id);
                    Some(item)
                },
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(_) => unimplemented!(),
            }
        } else {
            match self.db.query_row(
                "
                    SELECT rowid, command FROM command_history
                    WHERE command LIKE ? ESCAPE '\\'
                    ORDER BY rowid DESC
                ",
                params![self.pattern],
                Item::from_row,
            ) {
                Ok(item) => {
                    self.last_rowid = Some(item.id);
                    Some(item)
                },
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(_) => unimplemented!(),
            }
        }
    }
}

impl DoubleEndedIterator for Cursor {
    fn next_back(&mut self) -> Option<Item> {
        if let Some(last_rowid) = self.last_rowid.take() {
            match self.db.query_row(
                "
                    SELECT rowid, command FROM command_history
                    WHERE rowid > ? AND command LIKE ? ESCAPE '\\'
                    ORDER BY rowid DESC
                ",
                params![last_rowid, self.pattern],
                Item::from_row,
            ) {
                Ok(item) => {
                    self.last_rowid = Some(item.id);
                    Some(item)
                },
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(_) => unimplemented!(),
            }
        } else {
            // Already at the start.
            None
        }
    }
}

pub struct Item {
    id: i64,
    command: String,
}

impl Item {
    fn from_row(row: &Row) -> std::result::Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("rowid")?,
            command: row.get("command")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get() {
        let history = History::temporary().unwrap();

        for i in 0..9 {
            history.add(format!("echo {}", i));
        }

        let mut cursor = history.command_history(Sort::Chronological);

        for i in 0..9 {
            assert_eq!(cursor.next().unwrap().command, format!("echo {}", 8 - i));
        }
    }
}
