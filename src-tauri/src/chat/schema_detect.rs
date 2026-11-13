use std::collections::HashSet;

use rusqlite::Connection;

use super::error::{ChatError, ChatResult};
use super::schema::{
    foreign_key_cascades, has_column, require_columns, table_names, CleanupKind, CleanupTable,
    Schema, V1Tables, V2Tables, OPTIONAL_SESSION_TABLES, SESSION_COLUMNS, V1_MESSAGE,
    V1_MESSAGE_COLUMNS, V1_PART, V1_PART_COLUMNS, V1_SESSION, V2_MESSAGE, V2_MESSAGE_COLUMNS,
    V2_PART, V2_PART_COLUMNS, V2_SESSION,
};

/// Detects the v1 (`session`/`message`/`part`) and v2 (`session_v2`/`session_message`) schema.
///
/// Every recognised table must expose the fixed columns for its generation; anything else fails
/// closed so deletion can never run against an unknown layout. When both generations exist the
/// v1 tables stay primary because they hold the real dialogue, while v2 stays available for
/// sessions that only exist in the new layout.
pub fn introspect(conn: &Connection) -> ChatResult<Schema> {
    let tables = table_names(conn)?;
    let v1 = detect_v1(conn, &tables)?;
    let v2 = detect_v2(conn, &tables)?;
    let (generation, session_table) = primary(v1.as_ref(), v2.as_ref())?;
    let cleanup = cleanup_tables(v1.as_ref(), v2.as_ref());
    let cascaded = cascaded_tables(conn, &tables, &session_tables(v1.as_ref(), v2.as_ref()))?;
    Ok(Schema {
        generation,
        session_table,
        v1,
        v2,
        cleanup,
        cascaded,
    })
}

fn detect_v1(conn: &Connection, tables: &HashSet<String>) -> ChatResult<Option<V1Tables>> {
    if !tables.contains(V1_MESSAGE) {
        return Ok(None);
    }
    if !tables.contains(V1_SESSION) {
        return Err(ChatError::schema("存在 message 但缺少 session 表。"));
    }
    if !tables.contains(V1_PART) {
        return Err(ChatError::schema("v1 数据库缺少 part 表。"));
    }
    require_columns(conn, V1_SESSION, SESSION_COLUMNS)?;
    require_columns(conn, V1_MESSAGE, V1_MESSAGE_COLUMNS)?;
    require_columns(conn, V1_PART, V1_PART_COLUMNS)?;
    Ok(Some(V1Tables {
        session: V1_SESSION,
        message: V1_MESSAGE,
        part: V1_PART,
    }))
}

fn detect_v2(conn: &Connection, tables: &HashSet<String>) -> ChatResult<Option<V2Tables>> {
    if !tables.contains(V2_MESSAGE) {
        return Ok(None);
    }
    require_columns(conn, V2_MESSAGE, V2_MESSAGE_COLUMNS)?;
    let session = if tables.contains(V2_SESSION) {
        require_columns(conn, V2_SESSION, SESSION_COLUMNS)?;
        V2_SESSION
    } else if tables.contains(V1_SESSION) {
        require_columns(conn, V1_SESSION, SESSION_COLUMNS)?;
        V1_SESSION
    } else {
        return Err(ChatError::schema("存在 session_message 但缺少会话表。"));
    };
    let part = if tables.contains(V2_PART) {
        require_columns(conn, V2_PART, V2_PART_COLUMNS)?;
        Some(V2_PART)
    } else {
        None
    };
    Ok(Some(V2Tables {
        session,
        message: V2_MESSAGE,
        part,
    }))
}

fn primary(
    v1: Option<&V1Tables>,
    v2: Option<&V2Tables>,
) -> ChatResult<(&'static str, &'static str)> {
    match (v1, v2) {
        (Some(tables), _) => Ok(("v1", tables.session)),
        (None, Some(tables)) => Ok(("v2", tables.session)),
        (None, None) => Err(ChatError::schema(
            "未找到 session/message/session_message 表。",
        )),
    }
}

fn cleanup_tables(v1: Option<&V1Tables>, v2: Option<&V2Tables>) -> Vec<CleanupTable> {
    let mut cleanup = Vec::new();
    if v1.is_some() {
        cleanup.push(CleanupTable {
            table: V1_MESSAGE,
            kind: CleanupKind::Message,
        });
        cleanup.push(CleanupTable {
            table: V1_PART,
            kind: CleanupKind::Part,
        });
    }
    if let Some(tables) = v2 {
        cleanup.push(CleanupTable {
            table: tables.message,
            kind: CleanupKind::Message,
        });
        if tables.part == Some(V2_PART) {
            cleanup.push(CleanupTable {
                table: V2_PART,
                kind: CleanupKind::Part,
            });
        }
    }
    cleanup
}

fn session_tables(v1: Option<&V1Tables>, v2: Option<&V2Tables>) -> Vec<&'static str> {
    let mut names = Vec::new();
    if let Some(tables) = v1 {
        names.push(tables.session);
    }
    if let Some(tables) = v2 {
        if !names.contains(&tables.session) {
            names.push(tables.session);
        }
    }
    names
}

fn cascaded_tables(
    conn: &Connection,
    tables: &HashSet<String>,
    session_tables: &[&'static str],
) -> ChatResult<Vec<&'static str>> {
    let mut cascaded = Vec::new();
    for table in OPTIONAL_SESSION_TABLES {
        if !tables.contains(*table) || !has_column(conn, table, "session_id")? {
            continue;
        }
        for parent in session_tables {
            if foreign_key_cascades(conn, table, parent, "session_id")? {
                cascaded.push(*table);
                break;
            }
        }
    }
    Ok(cascaded)
}
