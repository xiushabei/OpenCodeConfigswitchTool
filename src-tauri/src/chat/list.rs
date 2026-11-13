use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params_from_iter, Connection};

use super::db::{as_u64, open_read_only, placeholders};
use super::error::{ChatError, ChatResult};
use super::schema::{introspect, Schema};
use super::types::{SessionList, SessionSummary, Sort};

/// Lists sessions with logical JSON sizes and message counts. Only allowlisted columns and
/// aggregate SQL are used; message payloads are never fetched into memory.
///
/// Each session reports the generation that actually holds its messages: v1 dialogue when the
/// `message` table has rows for it, otherwise the v2 tables. A database that carries both
/// generations therefore never hides real dialogue behind a handful of v2 control events.
///
/// # Errors
///
/// Returns [`ChatError`] when the database cannot be opened or its schema is not one of the two
/// supported generations.
pub fn list_sessions(path: &Path, sort: Sort, descending: bool) -> ChatResult<SessionList> {
    let conn = open_read_only(path)?;
    let schema = introspect(&conn)?;
    let sessions = query_sessions(&conn, &schema, sort, descending)?;
    Ok(SessionList {
        schema: schema.generation.to_string(),
        sessions,
        database_bytes: file_bytes(path),
        wal_bytes: file_bytes(&wal_path(path)),
        warnings: Vec::new(),
    })
}

fn file_bytes(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |meta| meta.len())
}

fn wal_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push("-wal");
    PathBuf::from(name)
}

fn query_sessions(
    conn: &Connection,
    schema: &Schema,
    sort: Sort,
    descending: bool,
) -> ChatResult<Vec<SessionSummary>> {
    let mut joins: Vec<String> = Vec::new();
    let mut v1_sources = None;
    let mut v2_sources = None;

    if let Some(tables) = &schema.v1 {
        joins.push(format!(
            "LEFT JOIN (SELECT session_id AS sid, COUNT(*) AS cnt, \
                      SUM(length(CAST(data AS BLOB))) AS bytes \
               FROM {} GROUP BY session_id) m ON m.sid = b.id",
            tables.message
        ));
        joins.push(format!(
            "LEFT JOIN (SELECT session_id AS sid, SUM(length(CAST(data AS BLOB))) AS bytes \
               FROM {} GROUP BY session_id) p ON p.sid = b.id",
            tables.part
        ));
        v1_sources = Some((
            "COALESCE(m.bytes, 0) + COALESCE(p.bytes, 0)".to_string(),
            "COALESCE(m.cnt, 0)".to_string(),
        ));
    }

    if let Some(tables) = &schema.v2 {
        joins.push(format!(
            "LEFT JOIN (SELECT session_id AS sid, COUNT(*) AS cnt, \
                      SUM(length(CAST(data AS BLOB))) AS bytes \
               FROM {} GROUP BY session_id) v ON v.sid = b.id",
            tables.message
        ));
        let mut size = vec!["COALESCE(v.bytes, 0)".to_string()];
        if let Some(part) = tables.part {
            joins.push(format!(
                "LEFT JOIN (SELECT session_id AS sid, SUM(length(CAST(data AS BLOB))) AS bytes \
                   FROM {part} GROUP BY session_id) sp ON sp.sid = b.id"
            ));
            size.push("COALESCE(sp.bytes, 0)".to_string());
        }
        v2_sources = Some((size.join(" + "), "COALESCE(v.cnt, 0)".to_string()));
    }

    let (size, count) = match (v1_sources, v2_sources) {
        (Some((v1_size, v1_count)), Some((v2_size, v2_count))) => (
            format!("CASE WHEN COALESCE(m.cnt, 0) > 0 THEN {v1_size} ELSE {v2_size} END"),
            format!("CASE WHEN COALESCE(m.cnt, 0) > 0 THEN {v1_count} ELSE {v2_count} END"),
        ),
        (Some((size, count)), None) | (None, Some((size, count))) => (size, count),
        (None, None) => return Err(ChatError::schema("未找到消息来源表。")),
    };
    let joins = joins.join(" ");
    let direction = if descending { "DESC" } else { "ASC" };
    let sql = format!(
        "SELECT s.id, s.title, s.directory, s.parent_id, s.created, s.updated, \
                s.size_bytes, s.message_count \
         FROM (SELECT b.id AS id, COALESCE(b.title, '') AS title, \
                      COALESCE(b.directory, '') AS directory, b.parent_id AS parent_id, \
                      b.time_created AS created, b.time_updated AS updated, \
                      {size} AS size_bytes, {count} AS message_count \
               FROM {session} b \
               {joins}) s \
         ORDER BY {column} {direction}, s.id ASC",
        session = schema.session_table,
        column = sort.column(),
    );
    let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map([], |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                directory: row.get(2)?,
                parent_id: row.get(3)?,
                created: row.get(4)?,
                updated: row.get(5)?,
                size_bytes: as_u64(row.get::<_, i64>(6)?),
                message_count: as_u64(row.get::<_, i64>(7)?),
            })
        })
        .map_err(|err| ChatError::query(&err))?;
    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row.map_err(|err| ChatError::query(&err))?);
    }
    Ok(sessions)
}

/// Logical byte total for a confirmed session set, used by the delete preview. Every session is
/// measured from the same source selected for reading, so the preview agrees with the list.
pub(crate) fn session_sizes(
    conn: &Connection,
    schema: &Schema,
    ids: &[String],
) -> ChatResult<HashMap<String, u64>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let marks = placeholders(ids.len());

    let mut v1_sizes = HashMap::new();
    if let Some(tables) = &schema.v1 {
        collect_sizes(conn, tables.message, &marks, ids, &mut v1_sizes)?;
        collect_sizes(conn, tables.part, &marks, ids, &mut v1_sizes)?;
    }

    let mut v2_sizes = HashMap::new();
    if let Some(tables) = &schema.v2 {
        collect_sizes(conn, tables.message, &marks, ids, &mut v2_sizes)?;
        if let Some(part) = tables.part {
            collect_sizes(conn, part, &marks, ids, &mut v2_sizes)?;
        }
    }

    let mut selected = HashMap::new();
    match &schema.v1 {
        Some(tables) => {
            let with_v1 = sessions_with_rows(conn, tables.message, &marks, ids)?;
            for id in ids {
                let source = if with_v1.contains(id) {
                    &v1_sizes
                } else {
                    &v2_sizes
                };
                selected.insert(id.clone(), source.get(id).copied().unwrap_or(0));
            }
        }
        None => {
            for id in ids {
                selected.insert(id.clone(), v2_sizes.get(id).copied().unwrap_or(0));
            }
        }
    }
    Ok(selected)
}

fn sessions_with_rows(
    conn: &Connection,
    table: &str,
    marks: &str,
    ids: &[String],
) -> ChatResult<HashSet<String>> {
    let sql = format!("SELECT DISTINCT session_id FROM {table} WHERE session_id IN ({marks})");
    let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map(params_from_iter(ids.iter()), |row| row.get::<_, String>(0))
        .map_err(|err| ChatError::query(&err))?;
    let mut present = HashSet::new();
    for row in rows {
        present.insert(row.map_err(|err| ChatError::query(&err))?);
    }
    Ok(present)
}

fn collect_sizes(
    conn: &Connection,
    table: &str,
    marks: &str,
    ids: &[String],
    sizes: &mut HashMap<String, u64>,
) -> ChatResult<()> {
    let sql = format!(
        "SELECT session_id, COALESCE(SUM(length(CAST(data AS BLOB))), 0) \
         FROM {table} WHERE session_id IN ({marks}) GROUP BY session_id"
    );
    let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map(params_from_iter(ids.iter()), |row| {
            Ok((row.get::<_, String>(0)?, as_u64(row.get::<_, i64>(1)?)))
        })
        .map_err(|err| ChatError::query(&err))?;
    for row in rows {
        let (id, bytes) = row.map_err(|err| ChatError::query(&err))?;
        *sizes.entry(id).or_insert(0) += bytes;
    }
    Ok(())
}
