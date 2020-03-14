use rusqlite::{params, Connection};
use std::{
    env,
    process,
    rc::Rc,
};

pub struct Session {
    db: Rc<Connection>,
    id: i64,
}

impl Session {
    pub(super) fn new(db: &Rc<Connection>) -> Self {
        db.execute(
            "INSERT INTO session_history (pid) VALUES (?)",
            params![process::id()],
        ).unwrap();

        Self {
            db: db.clone(),
            id: db.last_insert_rowid(),
        }
    }

    /// Record a command and add it to the history.
    pub fn add(&self, command: impl Into<String>) {
        let cwd = env::current_dir()
            .ok()
            .and_then(|path| path.to_str().map(String::from));

        self.db
            .execute(
                "INSERT INTO command_history (session_id, command, cwd) VALUES (?, ?, ?)",
                params![self.id, command.into(), cwd],
            )
            .unwrap();
    }
}
