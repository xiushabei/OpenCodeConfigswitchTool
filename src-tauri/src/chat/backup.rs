use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OpenFlags};

use super::db::BUSY_TIMEOUT;
use super::error::{ChatError, ChatResult};

/// Keeps concurrent backups in one process from choosing the same name when they start in the
/// same millisecond.
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

const MAX_ATTEMPTS: u32 = 100;

/// Writes a consistent, compact snapshot of the database into `backups/chat/` before any delete.
///
/// The snapshot is produced with `VACUUM INTO`, which copies every row, index and schema object
/// but skips SQLite free pages. A page-level copy through the online backup API reproduces the
/// source file 1:1, including the free pages left behind by earlier deletes; when a database has
/// been trimmed before, that is the dominant disk write of every delete. The compact snapshot
/// holds the same data and restores the same way.
///
/// The name carries the timestamp, process id and a process-wide sequence, so concurrent backups
/// never share it. The snapshot is `create_new`-reserved under a `.partial` name and renamed only
/// after SQLite finishes and the file is non-empty. The final name is never created before the
/// snapshot succeeds, so a failure leaves no empty file that looks like a usable backup.
///
/// # Errors
///
/// Returns [`ChatError::Backup`] when the backup directory or file cannot be created, when the
/// SQLite snapshot fails, or when it produces an empty file.
pub fn backup_database(path: &Path, root: &Path) -> ChatResult<String> {
    let directory = root.join("backups").join("chat");
    fs::create_dir_all(&directory).map_err(|err| ChatError::Backup {
        reason: format!("无法创建备份目录 {}：{err}", directory.display()),
    })?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| ChatError::Backup {
            reason: format!("系统时间无效：{err}"),
        })?
        .as_millis();
    let process = std::process::id();

    for _ in 0..MAX_ATTEMPTS {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = backup_name(stamp, process, sequence);
        let destination = directory.join(&name);
        match destination.try_exists() {
            Ok(false) => {}
            // A stale file (for example after pid reuse) keeps its name; pick the next sequence.
            Ok(true) => continue,
            Err(err) => {
                return Err(ChatError::Backup {
                    reason: format!("无法检查备份文件 {}：{err}", destination.display()),
                })
            }
        }

        // `VACUUM INTO` accepts an existing zero-length file but refuses a non-empty one, so a
        // `create_new` reservation is both the ownership lock and the snapshot destination.
        let partial = directory.join(format!("{name}.partial"));
        let reservation = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
        {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(ChatError::Backup {
                    reason: format!("无法创建备份文件 {}：{err}", partial.display()),
                })
            }
        };

        if let Err(err) = copy_into(path, &partial) {
            drop(reservation);
            let _ = fs::remove_file(&partial);
            return Err(err);
        }
        drop(reservation);

        // The unique name keeps this target free; re-check because Windows `fs::rename` replaces
        // an existing file instead of failing.
        match destination.try_exists() {
            Ok(false) => {}
            Ok(true) => {
                let _ = fs::remove_file(&partial);
                continue;
            }
            Err(err) => {
                let _ = fs::remove_file(&partial);
                return Err(ChatError::Backup {
                    reason: format!("无法完成备份文件 {}：{err}", destination.display()),
                });
            }
        }
        match fs::rename(&partial, &destination) {
            Ok(()) => return Ok(format!("backups/chat/{name}")),
            Err(err) => {
                let _ = fs::remove_file(&partial);
                if matches!(destination.try_exists(), Ok(true)) {
                    continue;
                }
                return Err(ChatError::Backup {
                    reason: format!("无法完成备份文件 {}：{err}", destination.display()),
                });
            }
        }
    }
    Err(ChatError::Backup {
        reason: "无法生成唯一的备份文件名。".to_string(),
    })
}

fn backup_name(stamp: u128, process: u32, sequence: u64) -> String {
    format!("chat-{stamp}-{process}-{sequence}.db")
}

fn copy_into(source: &Path, destination: &Path) -> ChatResult<()> {
    let destination_text = destination.to_str().ok_or_else(|| ChatError::Backup {
        reason: format!("备份路径不是有效的 UTF-8 路径：{}", destination.display()),
    })?;
    // `VACUUM INTO` only reads the source; the attached output database is opened separately by
    // SQLite, so a read-only source connection is enough.
    let conn =
        Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|err| {
            ChatError::Backup {
                reason: format!("无法打开聊天数据库进行备份：{err}"),
            }
        })?;
    conn.busy_timeout(BUSY_TIMEOUT)
        .map_err(|err| ChatError::Backup {
            reason: err.to_string(),
        })?;
    conn.execute("VACUUM INTO ?1", params![destination_text])
        .map_err(|err| ChatError::Backup {
            reason: err.to_string(),
        })?;
    let size = fs::metadata(destination)
        .map_err(|err| ChatError::Backup {
            reason: err.to_string(),
        })?
        .len();
    if size == 0 {
        return Err(ChatError::Backup {
            reason: "备份文件为空。".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::backup_name;

    #[test]
    fn concurrent_names_stay_distinct_in_one_millisecond() {
        let stamp = 1_700_000_000_000;
        let first = backup_name(stamp, 4242, 0);
        let second = backup_name(stamp, 4242, 1);
        assert_ne!(first, second);
        assert_ne!(backup_name(stamp, 4242, 0), backup_name(stamp, 4243, 0));
    }
}
