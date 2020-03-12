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

use rusqlite::{params, Connection, Row};
use std::{
    convert::TryFrom,
    env,
    error::Error,
    path::Path,
    process,
    rc::Rc,
    time::{Duration, UNIX_EPOCH},
};

mod entry;

pub use entry::{CommandEntry, EntryCursor};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// A connection to a history database.
#[derive(Clone)]
pub struct History {
    db: Rc<Connection>,
}

/// Aggregated information about a particular command string.
#[derive(Clone)]
pub struct CommandSummary {
    command: String,
    count: u32,
}

impl TryFrom<&Row<'_>> for CommandSummary {
    type Error = rusqlite::Error;

    fn try_from(row: &Row) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            command: row.get("command")?,
            count: row.get("count")?,
        })
    }
}

impl History {
    /// Open a history file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    /// Open a history file.
    pub fn open_default() -> Result<Self> {
        let dir = directories::ProjectDirs::from("sh.riptide", "", "Riptide")
            .unwrap()
            .data_dir()
            .to_owned();
        std::fs::create_dir_all(&dir)?;
        Self::open(dir.join("history.db"))
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
            1 => {}
            version => return Err(format!("unknown version: {}", version).into()),
        }

        Ok(history)
    }

    fn get_version(&self) -> i64 {
        self.db
            .query_row("PRAGMA user_version", params![], |row| row.get(0))
            .unwrap()
    }

    fn instrument(&self) -> Result<()> {
        self.db.execute_batch(
            "
            PRAGMA user_version = 1;

            CREATE TABLE command_history (
                command TEXT NOT NULL,
                cwd TEXT,
                pid INTEGER,
                timestamp INTEGER NOT NULL
            );
        ",
        )?;

        Ok(())
    }

    pub fn entries(&self) -> EntryCursor {
        EntryCursor::new(&self.db)
    }

    // /// Query for frequent commands.
    // pub fn frequent_commands(&self) -> Cursor<CommandSummary> {
    //     Cursor::query(
    //         &self.db,
    //         r#"
    //             SELECT command, count(*) AS count FROM command_history
    //             GROUP BY command
    //             ORDER BY count DESC
    //         "#,
    //         params![],
    //     )
    // }

    // /// Query for frequent commands with a prefix.
    // pub fn frequent_commands_starting_with(
    //     &self,
    //     prefix: impl Into<String>,
    // ) -> Cursor<CommandSummary> {
    //     let pattern = prefix.into().replace("%", "\\%").replace("\\", "\\\\") + "%";

    //     Cursor::query(
    //         &self.db,
    //         r#"
    //             SELECT command, count(*) AS count FROM command_history
    //             WHERE command LIKE ? ESCAPE "\"
    //             GROUP BY command
    //             ORDER BY count DESC
    //         "#,
    //         params![pattern],
    //     )
    // }

    /// Record a command and add it to the history.
    pub fn add(&self, command: impl Into<String>) {
        let cwd = env::current_dir()
            .ok()
            .and_then(|path| path.to_str().map(String::from));

        let pid = process::id();

        let timestamp = UNIX_EPOCH
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;

        self.db
            .execute(
                "INSERT INTO command_history (command, cwd, pid, timestamp) VALUES (?, ?, ?, ?)",
                params![command.into(), cwd, pid, timestamp],
            )
            .unwrap();
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

        let mut cursor = history.entries();

        for i in 0..9 {
            assert_eq!(cursor.next().unwrap().command(), format!("echo {}", 8 - i));
        }
    }
}
