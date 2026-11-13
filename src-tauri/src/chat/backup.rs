use std::fs::{self, OpenOptions};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::db::open_read_only;
use super::error::{ChatError, ChatResult};

/// Copies the database with SQLite's online backup API into `backups/chat/` before any delete.
///
/// # Errors
///
/// Returns [`ChatError::Backup`] when the backup directory or file cannot be created, or when the
/// SQLite backup fails or produces an empty file.
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

    for attempt in 0..100_u32 {
        let name = if attempt == 0 {
            format!("chat-{stamp}.db")
        } else {
            format!("chat-{stamp}-{attempt}.db")
        };
        let destination = directory.join(&name);
        let created = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination);
        if created.is_err() {
            continue;
        }
        drop(created);
        match copy_into(path, &destination) {
            Ok(()) => return Ok(format!("backups/chat/{name}")),
            Err(err) => {
                let _ = fs::remove_file(&destination);
                return Err(err);
            }
        }
    }
    Err(ChatError::Backup {
        reason: "无法生成唯一的备份文件名。".to_string(),
    })
}

fn copy_into(source: &Path, destination: &Path) -> ChatResult<()> {
    let conn = open_read_only(source)?;
    conn.backup("main", destination, None)
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
