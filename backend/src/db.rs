use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

/// Minimal SQLite-persistens for MVP: stabile fargetildelinger per navn,
/// slik at "Øystein" får gul igjen neste gang, pluss en enkel hendelseslogg.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS color_assignments (
                name TEXT PRIMARY KEY,
                color TEXT NOT NULL,
                last_seen INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                kind TEXT NOT NULL,
                participant TEXT NOT NULL
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn preferred_color(&self, name: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT color FROM color_assignments WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .ok()
    }

    pub fn remember_color(&self, name: &str, color: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO color_assignments (name, color, last_seen)
             VALUES (?1, ?2, strftime('%s','now'))
             ON CONFLICT(name) DO UPDATE SET color = ?2, last_seen = strftime('%s','now')",
            [name, color],
        );
    }

    pub fn log_event(&self, kind: &str, participant: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO events (ts, kind, participant) VALUES (strftime('%s','now'), ?1, ?2)",
            [kind, participant],
        );
    }
}
