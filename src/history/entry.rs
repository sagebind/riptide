use rusqlite::{params, Connection, Row};
use std::{
    convert::{TryFrom, TryInto},
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// A single entry in the history.
#[derive(Clone, Debug)]
pub struct CommandEntry {
    command: String,
    cwd: Option<String>,
    timestamp: SystemTime,
}

impl CommandEntry {
    pub fn command(&self) -> &str {
        self.command.as_str()
    }

    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
}

impl TryFrom<&Row<'_>> for CommandEntry {
    type Error = rusqlite::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            command: row.get("command")?,
            cwd: row.get("cwd")?,
            timestamp: UNIX_EPOCH + Duration::from_secs(row.get::<_, i64>("timestamp")? as u64),
        })
    }
}

/// An iterator over command history from newest to oldest.
pub struct EntryCursor {
    db: Rc<Connection>,
    key: Option<(i64, i64)>,
}

impl EntryCursor {
    pub(super) fn new(db: &Rc<Connection>) -> Self {
        Self {
            db: db.clone(),
            key: None,
        }
    }

    pub fn prev(&mut self) -> Option<CommandEntry> {
        if let Some((timestamp, rowid)) = self.key {
            let (timestamp, rowid, entry) = self.db.query_row(
                r#"
                    SELECT rowid, command, cwd, timestamp FROM command_history
                    WHERE (timestamp, rowid) > (?, ?)
                    ORDER BY timestamp ASC, rowid ASC
                    LIMIT 1
                "#,
                params![timestamp, rowid],
                |row| Ok((row.get("timestamp")?, row.get("rowid")?, row.try_into()?)),
            ).ok()?;

            self.key = Some((timestamp, rowid));

            Some(entry)
        } else {
            None
        }
    }
}

impl Iterator for EntryCursor {
    type Item = CommandEntry;

    fn next(&mut self) -> Option<CommandEntry> {
        let (timestamp, rowid, entry) = if let Some((timestamp, rowid)) = self.key {
            self.db.query_row(
                r#"
                    SELECT rowid, command, cwd, timestamp FROM command_history
                    WHERE (timestamp, rowid) < (?, ?)
                    ORDER BY timestamp DESC, rowid DESC
                    LIMIT 1
                "#,
                params![timestamp, rowid],
                |row| Ok((row.get("timestamp")?, row.get("rowid")?, row.try_into()?)),
            ).ok()?
        } else {
            self.db.query_row(
                r#"
                    SELECT rowid, command, cwd, timestamp FROM command_history
                    ORDER BY timestamp DESC, rowid DESC
                    LIMIT 1
                "#,
                params![],
                |row| Ok((row.get("timestamp")?, row.get("rowid")?, row.try_into()?)),
            ).ok()?
        };

        self.key = Some((timestamp, rowid));

        Some(entry)
    }
}
