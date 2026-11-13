use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use super::db::open_read_only;
use super::error::{ChatError, ChatResult};
use super::parse::{parse_v1_message, parse_v2_message, part_from_data};
use super::schema::{introspect, V1Tables, V2Tables};
use super::types::{MessageView, PartView, SessionRead};

/// Reads one session in full: every message, every part, every byte.
///
/// Ordering is v1 `time_created,id`, or v2 `seq` with `session_part.ordinal` (inline content
/// appended). Text is never sanitized, truncated, or capped; only malformed JSON is flagged.
///
/// A session keeps v1 as its source whenever the v1 `message` table has rows for it, even when a
/// coexisting `session_message` table holds a few control events. Only sessions without v1 rows
/// fall back to the v2 tables.
///
/// # Errors
///
/// Returns [`ChatError`] when the database cannot be opened, its schema is unknown, the session
/// does not exist, or a query fails.
pub fn read_session(path: &Path, session_id: &str) -> ChatResult<SessionRead> {
    let conn = open_read_only(path)?;
    let schema = introspect(&conn)?;
    let title = session_title(&conn, schema.session_table, session_id)?;
    let from_v1 = match &schema.v1 {
        Some(tables) => {
            let result = read_v1(&conn, tables, session_id)?;
            if result.0.is_empty() {
                None
            } else {
                Some(result)
            }
        }
        None => None,
    };
    let (messages, truncated) = match from_v1 {
        Some(result) => result,
        None => match &schema.v2 {
            Some(tables) => read_v2(&conn, tables, session_id)?,
            None => (Vec::new(), false),
        },
    };
    Ok(SessionRead {
        id: session_id.to_string(),
        title,
        messages,
        truncated,
        warnings: Vec::new(),
    })
}

fn session_title(conn: &Connection, session_table: &str, session_id: &str) -> ChatResult<String> {
    let sql = format!("SELECT COALESCE(title, '') FROM {session_table} WHERE id = ?1");
    conn.query_row(&sql, params![session_id], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|err| ChatError::query(&err))?
        .ok_or_else(|| ChatError::SessionNotFound {
            id: session_id.to_string(),
        })
}

fn read_v1(
    conn: &Connection,
    tables: &V1Tables,
    session_id: &str,
) -> ChatResult<(Vec<MessageView>, bool)> {
    let sql = format!(
        "SELECT id, time_created, CAST(data AS BLOB) FROM {} WHERE session_id = ?1 \
         ORDER BY time_created ASC, id ASC",
        tables.message
    );
    let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map(params![session_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(|err| ChatError::query(&err))?;
    let mut messages = Vec::new();
    for row in rows {
        messages.push(row.map_err(|err| ChatError::query(&err))?);
    }
    drop(statement);

    let part_sql = format!(
        "SELECT CAST(data AS BLOB) FROM {} WHERE message_id = ?1 ORDER BY time_created ASC, id ASC",
        tables.part
    );
    let mut part_statement = conn
        .prepare(&part_sql)
        .map_err(|err| ChatError::query(&err))?;
    let mut views = Vec::with_capacity(messages.len());
    let mut truncated = false;
    for (id, created, data) in messages {
        let parsed = parse_v1_message(&String::from_utf8_lossy(&data));
        let parts = load_v1_parts(&mut part_statement, &id)?;
        let malformed = parsed.truncated || parts.iter().any(|part| part.truncated);
        truncated |= malformed;
        views.push(MessageView {
            id,
            role: parsed.role,
            created: Some(created),
            parts,
        });
    }
    Ok((views, truncated))
}

fn read_v2(
    conn: &Connection,
    tables: &V2Tables,
    session_id: &str,
) -> ChatResult<(Vec<MessageView>, bool)> {
    let sql = format!(
        "SELECT id, type, time_created, CAST(data AS BLOB) FROM {} WHERE session_id = ?1 \
         ORDER BY seq ASC, id ASC",
        tables.message
    );
    let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map(params![session_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        })
        .map_err(|err| ChatError::query(&err))?;
    let mut messages = Vec::new();
    for row in rows {
        messages.push(row.map_err(|err| ChatError::query(&err))?);
    }
    drop(statement);

    let mut part_statement = match tables.part {
        Some(part_table) => {
            let part_sql = format!(
                "SELECT type, CAST(data AS BLOB) FROM {part_table} WHERE message_id = ?1 ORDER BY ordinal ASC"
            );
            Some(
                conn.prepare(&part_sql)
                    .map_err(|err| ChatError::query(&err))?,
            )
        }
        None => None,
    };
    let mut views = Vec::with_capacity(messages.len());
    let mut truncated = false;
    for (id, kind, created, data) in messages {
        let mut parsed = parse_v2_message(&String::from_utf8_lossy(&data), &kind);
        if let Some(statement) = part_statement.as_mut() {
            let mut stored = load_v2_parts(statement, &id)?;
            stored.append(&mut parsed.parts);
            parsed.parts = stored;
        }
        let malformed = parsed.truncated || parsed.parts.iter().any(|part| part.truncated);
        truncated |= malformed;
        views.push(MessageView {
            id,
            role: parsed.role,
            created: Some(created),
            parts: parsed.parts,
        });
    }
    Ok((views, truncated))
}

fn load_v1_parts(
    statement: &mut rusqlite::Statement<'_>,
    message_id: &str,
) -> ChatResult<Vec<PartView>> {
    let rows = statement
        .query_map(params![message_id], |row| row.get::<_, Vec<u8>>(0))
        .map_err(|err| ChatError::query(&err))?;
    let mut parts = Vec::new();
    for row in rows {
        let data = row.map_err(|err| ChatError::query(&err))?;
        parts.push(part_from_data(&String::from_utf8_lossy(&data), None));
    }
    Ok(parts)
}

fn load_v2_parts(
    statement: &mut rusqlite::Statement<'_>,
    message_id: &str,
) -> ChatResult<Vec<PartView>> {
    let rows = statement
        .query_map(params![message_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(|err| ChatError::query(&err))?;
    let mut parts = Vec::new();
    for row in rows {
        let (kind, data) = row.map_err(|err| ChatError::query(&err))?;
        parts.push(part_from_data(&String::from_utf8_lossy(&data), Some(&kind)));
    }
    Ok(parts)
}
