use std::path::Path;

use rusqlite::{params_from_iter, Connection, TransactionBehavior};

use super::backup::backup_database;
use super::db::{open_read_only, open_read_write, placeholders};
use super::descendants::{subtree, titles};
use super::error::{ChatError, ChatResult};
use super::list::session_sizes;
use super::schema::{introspect, verify_deletion, CleanupKind, Schema};
use super::types::{DeletePreview, DeleteReport};

/// Recomputes the full descendant set and its logical size without touching the database.
///
/// # Errors
///
/// Returns [`ChatError`] when the database cannot be opened, its schema is unknown, or the root
/// session does not exist.
pub fn preview(path: &Path, session_id: &str) -> ChatResult<DeletePreview> {
    let conn = open_read_only(path)?;
    let schema = introspect(&conn)?;
    let ids = subtree(&conn, &schema, session_id)?;
    let titles = titles(&conn, &schema, &ids)?;
    let sizes = session_sizes(&conn, &schema, &ids)?;
    let total = ids
        .iter()
        .map(|id| sizes.get(id).copied().unwrap_or(0))
        .sum();
    Ok(DeletePreview {
        session_ids: ids,
        titles,
        size_bytes: total,
        blocked_reason: None,
    })
}

/// Deletes the confirmed subtree. The caller must run the process check immediately before this.
///
/// Preflight recomputes the descendant set, then backs the database up through SQLite's online
/// backup API. Only after that does it open a write transaction and remove verified child rows.
///
/// # Errors
///
/// Returns [`ChatError::Confirmation`] when the confirmed set differs from the recomputed
/// descendants, [`ChatError::Backup`] when the backup fails, [`ChatError::Schema`] for an unknown
/// layout, or [`ChatError::Query`] when the transaction fails and rolls back.
pub fn execute(
    path: &Path,
    root: &Path,
    session_id: &str,
    confirmed: &[String],
) -> ChatResult<DeleteReport> {
    let mut conn = open_read_write(path)?;
    let schema = introspect(&conn)?;
    verify_deletion(&conn, &schema)?;
    let ids = subtree(&conn, &schema, session_id)?;
    confirm_matches(&ids, confirmed)?;
    let backup = backup_database(path, root)?;
    let deleted = remove_rows(&mut conn, &schema, &ids)?;
    let checkpoint = checkpoint(&conn);
    Ok(DeleteReport {
        deleted_session_ids: ids.clone(),
        backup: backup.clone(),
        message: format!(
            "已删除 {deleted} 个会话（含全部子会话）。数据库备份：{backup}。\
             SQLite 将在后续操作中回收物理文件空间。{checkpoint}"
        ),
    })
}

fn confirm_matches(expected: &[String], confirmed: &[String]) -> ChatResult<()> {
    let mut expected = expected.to_vec();
    expected.sort();
    let mut confirmed = confirmed.to_vec();
    confirmed.sort();
    confirmed.dedup();
    if confirmed == expected {
        return Ok(());
    }
    Err(ChatError::Confirmation {
        expected,
        confirmed,
    })
}

fn remove_rows(conn: &mut Connection, schema: &Schema, ids: &[String]) -> ChatResult<usize> {
    let marks = placeholders(ids.len());
    let transaction = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| ChatError::query(&err))?;
    for kind in [CleanupKind::Part, CleanupKind::Message] {
        for table in schema.cleanup.iter().filter(|item| item.kind == kind) {
            let sql = format!("DELETE FROM {} WHERE session_id IN ({marks})", table.table);
            transaction
                .execute(&sql, params_from_iter(ids.iter()))
                .map_err(|err| ChatError::query(&err))?;
        }
    }
    for table in &schema.cascaded {
        let sql = format!("DELETE FROM {table} WHERE session_id IN ({marks})");
        transaction
            .execute(&sql, params_from_iter(ids.iter()))
            .map_err(|err| ChatError::query(&err))?;
    }
    let sql = format!("DELETE FROM {} WHERE id IN ({marks})", schema.session_table);
    let deleted = transaction
        .execute(&sql, params_from_iter(ids.iter()))
        .map_err(|err| ChatError::query(&err))?;
    if deleted != ids.len() {
        return Err(ChatError::schema(
            "删除的会话数量与预期不一致，事务已回滚。",
        ));
    }
    transaction.commit().map_err(|err| ChatError::query(&err))?;
    Ok(deleted)
}

fn checkpoint(conn: &Connection) -> String {
    match conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
        row.get::<_, i64>(0)
    }) {
        Ok(0) => "WAL 检查点已完成。".to_string(),
        Ok(_) => "WAL 检查点因数据库繁忙未完成。".to_string(),
        Err(err) => format!("WAL 检查点未完成：{err}。"),
    }
}
