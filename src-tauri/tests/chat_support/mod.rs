#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

/// Synthetic, project-local database workspace removed on drop. Never points at a real profile.
pub struct Fixture {
    root: PathBuf,
    database: PathBuf,
}

impl Fixture {
    pub fn new(name: &str) -> Self {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-work");
        fs::create_dir_all(&base).expect("create test-work directory");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let root = base.join(format!("{name}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&root).expect("create chat fixture root");
        let database = root.join("opencode.db");
        Self { root, database }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> &Path {
        &self.database
    }

    pub fn connection(&self) -> Connection {
        Connection::open(&self.database).expect("open synthetic database")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn create_v1(fixture: &Fixture) -> Connection {
    let conn = fixture.connection();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE session (
           id TEXT PRIMARY KEY,
           title TEXT NOT NULL,
           directory TEXT NOT NULL,
           parent_id TEXT,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL
         );
         CREATE TABLE message (
           id TEXT PRIMARY KEY,
           session_id TEXT NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL,
           data TEXT NOT NULL,
           FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE CASCADE
         );
         CREATE TABLE part (
           id TEXT PRIMARY KEY,
           message_id TEXT NOT NULL,
           session_id TEXT NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL,
           data TEXT NOT NULL,
           FOREIGN KEY (message_id) REFERENCES message(id) ON DELETE CASCADE
         );
         CREATE INDEX session_parent_idx ON session(parent_id);
         CREATE INDEX message_session_time_idx ON message(session_id, time_created, id);
         CREATE INDEX part_message_time_idx ON part(message_id, time_created, id);
         CREATE INDEX part_session_idx ON part(session_id);",
    )
    .expect("create v1 schema");
    conn
}

pub fn create_v2(fixture: &Fixture) -> Connection {
    let conn = fixture.connection();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE session_v2 (
           id TEXT PRIMARY KEY,
           title TEXT,
           directory TEXT NOT NULL,
           parent_id TEXT,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL
         );
         CREATE TABLE session_message (
           id TEXT PRIMARY KEY,
           session_id TEXT NOT NULL,
           type TEXT NOT NULL,
           seq INTEGER NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL,
           data TEXT NOT NULL,
           FOREIGN KEY (session_id) REFERENCES session_v2(id) ON DELETE CASCADE
         );
         CREATE TABLE session_part (
           message_id TEXT NOT NULL,
           session_id TEXT NOT NULL,
           ordinal INTEGER NOT NULL,
           type TEXT NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL,
           data TEXT NOT NULL,
           PRIMARY KEY (message_id, ordinal),
           FOREIGN KEY (message_id) REFERENCES session_message(id) ON DELETE CASCADE,
           FOREIGN KEY (session_id) REFERENCES session_v2(id) ON DELETE CASCADE
         );
         CREATE TABLE blob_ref (
           session_id TEXT NOT NULL,
           message_id TEXT NOT NULL,
           hash TEXT NOT NULL,
           seq INTEGER NOT NULL,
           PRIMARY KEY (session_id, message_id, hash),
           FOREIGN KEY (session_id) REFERENCES session_v2(id) ON DELETE CASCADE
         );
         CREATE TABLE session_inbox (
           id TEXT PRIMARY KEY,
           session_id TEXT NOT NULL,
           type TEXT NOT NULL,
           payload TEXT NOT NULL,
           delivery TEXT NOT NULL,
           enqueued_seq INTEGER NOT NULL,
           time_created INTEGER NOT NULL,
           FOREIGN KEY (session_id) REFERENCES session_v2(id) ON DELETE CASCADE
         );
         CREATE INDEX session_v2_parent_idx ON session_v2(parent_id);
         CREATE INDEX session_message_session_seq_idx ON session_message(session_id, seq);
         CREATE INDEX session_part_message_ordinal_idx ON session_part(message_id, ordinal);
         CREATE INDEX session_part_session_idx ON session_part(session_id);
         CREATE INDEX blob_ref_session_idx ON blob_ref(session_id);
         CREATE INDEX session_inbox_session_idx ON session_inbox(session_id);",
    )
    .expect("create v2 schema");
    conn
}

pub fn insert_session(
    conn: &Connection,
    id: &str,
    title: &str,
    parent: Option<&str>,
    created: i64,
    updated: i64,
) {
    let table = if has_table(conn, "session_v2") {
        "session_v2"
    } else {
        "session"
    };
    let sql = format!(
        "INSERT INTO {table} (id, title, directory, parent_id, time_created, time_updated) \
         VALUES (?1, ?2, '/work', ?3, ?4, ?5)"
    );
    conn.execute(&sql, params![id, title, parent, created, updated])
        .expect("insert session");
}

pub fn insert_v1_message(conn: &Connection, id: &str, session: &str, created: i64, data: &str) {
    conn.execute(
        "INSERT INTO message (id, session_id, time_created, time_updated, data) \
         VALUES (?1, ?2, ?3, ?3, ?4)",
        params![id, session, created, data],
    )
    .expect("insert message");
}

pub fn insert_v1_part(
    conn: &Connection,
    id: &str,
    message: &str,
    session: &str,
    created: i64,
    data: &str,
) {
    conn.execute(
        "INSERT INTO part (id, message_id, session_id, time_created, time_updated, data) \
         VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
        params![id, message, session, created, data],
    )
    .expect("insert part");
}

pub fn insert_v2_message(
    conn: &Connection,
    id: &str,
    session: &str,
    kind: &str,
    seq: i64,
    data: &str,
) {
    conn.execute(
        "INSERT INTO session_message (id, session_id, type, seq, time_created, time_updated, data) \
         VALUES (?1, ?2, ?3, ?4, ?4, ?4, ?5)",
        params![id, session, kind, seq, data],
    )
    .expect("insert session_message");
}

pub fn insert_v2_part(
    conn: &Connection,
    message: &str,
    session: &str,
    ordinal: i64,
    kind: &str,
    data: &str,
) {
    conn.execute(
        "INSERT INTO session_part \
           (message_id, session_id, ordinal, type, time_created, time_updated, data) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
        params![message, session, ordinal, kind, ordinal * 10, data],
    )
    .expect("insert session_part");
}

pub fn insert_blob_ref(conn: &Connection, session: &str, message: &str, hash: &str) {
    conn.execute(
        "INSERT INTO blob_ref (session_id, message_id, hash, seq) VALUES (?1, ?2, ?3, 1)",
        params![session, message, hash],
    )
    .expect("insert blob_ref");
}

pub fn insert_inbox(conn: &Connection, id: &str, session: &str) {
    conn.execute(
        "INSERT INTO session_inbox \
           (id, session_id, type, payload, delivery, enqueued_seq, time_created) \
         VALUES (?1, ?2, 'text', '{}', 'immediate', 1, 1)",
        params![id, session],
    )
    .expect("insert session_inbox");
}

pub fn count(conn: &Connection, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    conn.query_row(&sql, [], |row| row.get(0))
        .expect("count rows")
}

pub fn has_table(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .expect("query sqlite_master")
        > 0
}
