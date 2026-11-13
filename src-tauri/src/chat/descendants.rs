use std::collections::HashSet;

use rusqlite::{params, params_from_iter, Connection, OptionalExtension};

use super::db::placeholders;
use super::error::{ChatError, ChatResult};
use super::schema::Schema;

const MAX_SUBTREE: usize = 10_000;

/// Returns the root and every descendant by `parent_id`, with a cycle guard and a hard cap.
pub fn subtree(conn: &Connection, schema: &Schema, root: &str) -> ChatResult<Vec<String>> {
    let exists_sql = format!("SELECT 1 FROM {} WHERE id = ?1", schema.session_table);
    let exists = conn
        .query_row(&exists_sql, params![root], |_| Ok(()))
        .optional()
        .map_err(|err| ChatError::query(&err))?;
    if exists.is_none() {
        return Err(ChatError::SessionNotFound {
            id: root.to_string(),
        });
    }

    let mut order = vec![root.to_string()];
    let mut seen = HashSet::from([root.to_string()]);
    let mut frontier = vec![root.to_string()];
    while !frontier.is_empty() {
        let marks = placeholders(frontier.len());
        let sql = format!(
            "SELECT id FROM {} WHERE parent_id IN ({marks}) ORDER BY id ASC",
            schema.session_table
        );
        let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
        let rows = statement
            .query_map(params_from_iter(frontier.iter()), |row| {
                row.get::<_, String>(0)
            })
            .map_err(|err| ChatError::query(&err))?;
        let mut next = Vec::new();
        for row in rows {
            let id = row.map_err(|err| ChatError::query(&err))?;
            if seen.insert(id.clone()) {
                order.push(id.clone());
                next.push(id);
                if order.len() > MAX_SUBTREE {
                    return Err(ChatError::Limit {
                        reason: format!("会话树超过 {MAX_SUBTREE} 个节点。"),
                    });
                }
            }
        }
        frontier = next;
    }
    Ok(order)
}

/// Titles aligned with `ids`; missing rows report an empty title rather than aborting a preview.
pub fn titles(conn: &Connection, schema: &Schema, ids: &[String]) -> ChatResult<Vec<String>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let marks = placeholders(ids.len());
    let sql = format!(
        "SELECT id, COALESCE(title, '') FROM {} WHERE id IN ({marks})",
        schema.session_table
    );
    let mut statement = conn.prepare(&sql).map_err(|err| ChatError::query(&err))?;
    let rows = statement
        .query_map(params_from_iter(ids.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| ChatError::query(&err))?;
    let mut titles = std::collections::HashMap::new();
    for row in rows {
        let (id, title) = row.map_err(|err| ChatError::query(&err))?;
        titles.insert(id, title);
    }
    Ok(ids
        .iter()
        .map(|id| titles.get(id).cloned().unwrap_or_default())
        .collect())
}
