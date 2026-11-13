use std::path::PathBuf;

use tauri::State;

use crate::chat::{
    self, ChatError, DeletePreview, DeleteReport, DiscoveryResult, SessionList, SessionRead, Sort,
};
use crate::engine::Engine;

/// Locates candidate chat databases. A full-drive scan is cached per process and only runs when
/// the environment, CLI, and default paths all miss.
///
/// # Errors
///
/// Returns a user-facing message when the blocking task cannot be joined.
#[tauri::command]
pub async fn discover_chat_databases() -> Result<DiscoveryResult, String> {
    tauri::async_runtime::spawn_blocking(chat::discover)
        .await
        .map_err(|err| format!("聊天数据库发现任务异常结束：{err}"))
}

/// Lists sessions with logical JSON sizes and message counts.
///
/// # Errors
///
/// Returns a user-facing message for an invalid sort key, an unreadable or unrecognised database,
/// or a failed blocking task.
#[tauri::command]
pub async fn list_chat_sessions(
    path: String,
    sort: String,
    descending: bool,
) -> Result<SessionList, String> {
    let sort = Sort::parse(&sort).ok_or_else(|| format!("排序方式无效：{sort}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        chat::list_sessions(&PathBuf::from(path), sort, descending)
    })
    .await
    .map_err(|err| format!("会话列表任务异常结束：{err}"))?
    .map_err(|err| err.to_string())
}

/// Reads one session with bounded text, tool summaries, and no raw base64 payloads.
///
/// # Errors
///
/// Returns a user-facing message when the session or database cannot be read, or the blocking task
/// fails.
#[tauri::command]
pub async fn read_chat_session(path: String, session_id: String) -> Result<SessionRead, String> {
    tauri::async_runtime::spawn_blocking(move || {
        chat::read_session(&PathBuf::from(path), &session_id)
    })
    .await
    .map_err(|err| format!("会话内容任务异常结束：{err}"))?
    .map_err(|err| err.to_string())
}

/// Recomputes the descendant set, its size, and the live process block state.
///
/// # Errors
///
/// Returns a user-facing message when the database cannot be read or the blocking task fails.
#[tauri::command]
pub async fn preview_chat_delete(
    path: String,
    session_id: String,
) -> Result<DeletePreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut preview = chat::preview_delete(&PathBuf::from(path), &session_id)?;
        preview.blocked_reason = match chat::opencode_running() {
            Ok(true) => Some("检测到 OpenCode 正在运行，请完全退出后再删除。".to_string()),
            Ok(false) => None,
            Err(err) => Some(format!("无法确认 OpenCode 是否正在运行，已阻止删除：{err}")),
        };
        Ok::<_, ChatError>(preview)
    })
    .await
    .map_err(|err| format!("删除预览任务异常结束：{err}"))?
    .map_err(|err| err.to_string())
}

/// Deletes a confirmed session subtree after a second process check and SQLite online backup.
///
/// # Errors
///
/// Returns a user-facing message when `OpenCode` is running or the check fails, when the confirmed
/// set no longer matches the recomputed descendants, or when the backup or transaction fails.
#[tauri::command]
pub async fn delete_chat_session(
    path: String,
    session_id: String,
    confirmed_session_ids: Vec<String>,
    engine: State<'_, Engine>,
) -> Result<DeleteReport, String> {
    let root = engine.root().to_path_buf();
    tauri::async_runtime::spawn_blocking(move || {
        if chat::opencode_running()? {
            return Err(ChatError::Blocked {
                reason: "检测到 OpenCode 进程，删除已取消。".to_string(),
            });
        }
        chat::execute_delete(
            &PathBuf::from(path),
            &root,
            &session_id,
            &confirmed_session_ids,
        )
    })
    .await
    .map_err(|err| format!("删除任务异常结束：{err}"))?
    .map_err(|err| err.to_string())
}
