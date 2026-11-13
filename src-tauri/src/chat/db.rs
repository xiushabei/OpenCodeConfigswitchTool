use std::fs;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use super::error::{ChatError, ChatResult};

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Validates that `path` points at an existing database file without creating anything.
pub fn validate_path(path: &Path) -> ChatResult<()> {
    let metadata = fs::metadata(path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => ChatError::NotFound {
            path: path.display().to_string(),
        },
        _ => ChatError::InvalidPath {
            reason: format!("{}（{err}）", path.display()),
        },
    })?;
    if !metadata.is_file() {
        return Err(ChatError::InvalidPath {
            reason: format!("{} 不是文件。", path.display()),
        });
    }
    Ok(())
}

/// Opens the database read-only with `query_only` enabled. WAL mode is left untouched so a live
/// database stays readable without `immutable=1`.
pub fn open_read_only(path: &Path) -> ChatResult<Connection> {
    validate_path(path)?;
    let conn =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|err| {
            ChatError::Open {
                path: path.display().to_string(),
                reason: err.to_string(),
            }
        })?;
    conn.busy_timeout(BUSY_TIMEOUT)
        .map_err(|err| ChatError::query(&err))?;
    conn.pragma_update(None, "query_only", "ON")
        .map_err(|err| ChatError::query(&err))?;
    conn.pragma_update(None, "trusted_schema", "OFF")
        .map_err(|err| ChatError::query(&err))?;
    Ok(conn)
}

/// Opens the database read-write without create permission and proves foreign keys are enabled.
pub fn open_read_write(path: &Path) -> ChatResult<Connection> {
    validate_path(path)?;
    let conn =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|err| {
            ChatError::Open {
                path: path.display().to_string(),
                reason: err.to_string(),
            }
        })?;
    conn.busy_timeout(BUSY_TIMEOUT)
        .map_err(|err| ChatError::query(&err))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|err| ChatError::query(&err))?;
    let enabled = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
        .map_err(|err| ChatError::query(&err))?;
    if enabled != 1 {
        return Err(ChatError::schema("无法启用 foreign_keys，已拒绝写入。"));
    }
    Ok(conn)
}

/// Builds `?,?,...` for an allowlisted `IN` clause. Callers guard against an empty slice.
pub fn placeholders(count: usize) -> String {
    let mut sql = String::with_capacity(count.saturating_mul(2));
    for index in 0..count {
        if index > 0 {
            sql.push(',');
        }
        sql.push('?');
    }
    sql
}

/// Converts a SQLite integer aggregate into a non-negative byte count.
pub fn as_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}
