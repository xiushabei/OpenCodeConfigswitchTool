use std::fs;
use std::io;
use std::path::Path;

use crate::error::AppError;
use crate::model::{Target, BACKUPS_DIR};
use crate::storage;

/// Copies the exact original bytes into `backups/` using an exclusive timestamped file.
///
/// # Errors
///
/// Returns [`AppError`] when the backup directory cannot be created or no exclusive backup file
/// can be written.
pub fn write_backup(root: &Path, target: Target, bytes: &[u8]) -> Result<String, AppError> {
    let dir = root.join(BACKUPS_DIR);
    fs::create_dir_all(&dir).map_err(|err| AppError::io("创建备份目录", &dir, err))?;

    for attempt in 0..100 {
        let name = backup_name(target, attempt);
        let path = dir.join(&name);
        match storage::try_create_file(&path, bytes) {
            Ok(()) => return Ok(format!("{BACKUPS_DIR}/{name}")),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(AppError::io("写入备份文件", &path, err)),
        }
    }
    Err(AppError::io(
        "写入备份文件",
        &dir,
        io::Error::new(io::ErrorKind::AlreadyExists, "无法生成唯一的备份文件名"),
    ))
}

fn backup_name(target: Target, attempt: u32) -> String {
    let stamp = storage::timestamp_millis();
    let stem = target.as_str();
    if attempt == 0 {
        format!("{stem}-{stamp}.json")
    } else {
        format!("{stem}-{stamp}-{attempt}.json")
    }
}
