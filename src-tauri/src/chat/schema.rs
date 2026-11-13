use std::collections::HashSet;

use rusqlite::Connection;

use super::error::{ChatError, ChatResult};

pub use super::schema_detect::introspect;

pub const V1_SESSION: &str = "session";
pub const V1_MESSAGE: &str = "message";
pub const V1_PART: &str = "part";
pub const V2_SESSION: &str = "session_v2";
pub const V2_MESSAGE: &str = "session_message";
pub const V2_PART: &str = "session_part";

pub(super) const SESSION_COLUMNS: &[&str] = &[
    "id",
    "title",
    "directory",
    "parent_id",
    "time_created",
    "time_updated",
];
pub(super) const V1_MESSAGE_COLUMNS: &[&str] = &["id", "session_id", "time_created", "data"];
pub(super) const V1_PART_COLUMNS: &[&str] =
    &["id", "message_id", "session_id", "time_created", "data"];
pub(super) const V2_MESSAGE_COLUMNS: &[&str] =
    &["id", "session_id", "type", "seq", "time_created", "data"];
pub(super) const V2_PART_COLUMNS: &[&str] =
    &["message_id", "session_id", "ordinal", "type", "data"];

/// Tables that may reference a session and are only touched when their FK cascade is verified.
pub(super) const OPTIONAL_SESSION_TABLES: &[&str] = &[
    "blob_ref",
    "session_inbox",
    "session_context_epoch",
    "session_input",
    "session_entry",
    "session_pending",
    "schedule",
    "instruction_entry",
    "instruction_state",
    "todo",
    "session_share",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupKind {
    Message,
    Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CleanupTable {
    pub table: &'static str,
    pub kind: CleanupKind,
}

/// The complete v1 table set. `part` is mandatory for a v1 database.
#[derive(Clone, Copy, Debug)]
pub struct V1Tables {
    pub session: &'static str,
    pub message: &'static str,
    pub part: &'static str,
}

/// The complete v2 table set. `session_part` is optional so the legacy layout that kept
/// `session`/`session_message` stays readable.
#[derive(Clone, Copy, Debug)]
pub struct V2Tables {
    pub session: &'static str,
    pub message: &'static str,
    pub part: Option<&'static str>,
}

/// Column allowlists proven by introspection. Nothing here is taken from user input.
///
/// A live database may carry both generations at once (v1 dialogue plus a few v2 control
/// events). `generation` is the primary source reported to the UI; per-session reads still
/// fall back to the other set when the primary set has no rows for that session.
#[derive(Clone, Debug)]
pub struct Schema {
    pub generation: &'static str,
    pub session_table: &'static str,
    pub v1: Option<V1Tables>,
    pub v2: Option<V2Tables>,
    pub cleanup: Vec<CleanupTable>,
    pub cascaded: Vec<&'static str>,
}

/// Deletion-only guard: browsing tolerates missing FK clauses, but deletion requires the child
/// relationships to cascade so an unknown layout can never strand or mis-delete rows. Every
/// cleanup table present in the database is verified, including the generation that is not
/// primary for reading.
pub fn verify_deletion(conn: &Connection, schema: &Schema) -> ChatResult<()> {
    for item in &schema.cleanup {
        let (parent, column) = match (item.table, item.kind) {
            (V1_MESSAGE, CleanupKind::Message) => (V1_SESSION, "session_id"),
            (V1_PART, CleanupKind::Part) => (V1_MESSAGE, "message_id"),
            (V2_MESSAGE, CleanupKind::Message) => (v2_session_table(schema)?, "session_id"),
            (V2_PART, CleanupKind::Part) => (V2_MESSAGE, "message_id"),
            (other, _) => {
                return Err(ChatError::schema(format!(
                    "未验证的清理表 {other}，已拒绝删除。"
                )))
            }
        };
        require_cascade(conn, item.table, parent, column)?;
        if item.table == V2_PART {
            require_cascade(conn, V2_PART, v2_session_table(schema)?, "session_id")?;
        }
    }
    Ok(())
}

fn v2_session_table(schema: &Schema) -> ChatResult<&'static str> {
    schema
        .v2
        .as_ref()
        .map(|tables| tables.session)
        .ok_or_else(|| ChatError::schema("缺少 v2 会话表，已拒绝删除。"))
}

fn require_cascade(conn: &Connection, table: &str, parent: &str, column: &str) -> ChatResult<()> {
    if foreign_key_cascades(conn, table, parent, column)? {
        return Ok(());
    }
    Err(ChatError::schema(format!(
        "表 {table} 缺少到 {parent} 的 ON DELETE CASCADE 外键，已拒绝删除。"
    )))
}

pub fn table_names(conn: &Connection) -> ChatResult<HashSet<String>> {
    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| ChatError::query(&err))?;
    let mut names = HashSet::new();
    for row in rows {
        names.insert(row.map_err(|err| ChatError::query(&err))?);
    }
    Ok(names)
}

pub fn columns(conn: &Connection, table: &str) -> ChatResult<HashSet<String>> {
    let sql = format!("PRAGMA table_info(\"{table}\")");
    let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| ChatError::query(&err))?;
    let mut names = HashSet::new();
    for row in rows {
        names.insert(row.map_err(|err| ChatError::query(&err))?);
    }
    Ok(names)
}

pub fn has_column(conn: &Connection, table: &str, column: &str) -> ChatResult<bool> {
    Ok(columns(conn, table)?.contains(column))
}

pub(super) fn require_columns(conn: &Connection, table: &str, required: &[&str]) -> ChatResult<()> {
    let present = columns(conn, table)?;
    for column in required {
        if !present.contains(*column) {
            return Err(ChatError::schema(format!(
                "表 {table} 缺少固定列 {column}。"
            )));
        }
    }
    Ok(())
}

pub(super) fn foreign_key_cascades(
    conn: &Connection,
    table: &str,
    parent: &str,
    column: &str,
) -> ChatResult<bool> {
    let sql = format!("PRAGMA foreign_key_list(\"{table}\")");
    let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|err| ChatError::query(&err))?;
    for row in rows {
        let (target, from, on_delete) = row.map_err(|err| ChatError::query(&err))?;
        if target == parent && from == column && on_delete.eq_ignore_ascii_case("CASCADE") {
            return Ok(true);
        }
    }
    Ok(false)
}
