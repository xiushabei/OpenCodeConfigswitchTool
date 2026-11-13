use serde::{Deserialize, Serialize};

/// Result of locating candidate `OpenCode` databases.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    pub paths: Vec<String>,
    pub searched_all_drives: bool,
    pub warnings: Vec<String>,
}

/// One session row plus its content JSON byte total and message count.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub directory: String,
    pub parent_id: Option<String>,
    pub created: i64,
    pub updated: i64,
    pub size_bytes: u64,
    pub message_count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionList {
    pub schema: String,
    pub sessions: Vec<SessionSummary>,
    /// Actual byte length of the main database file.
    pub database_bytes: u64,
    /// Actual byte length of the separate WAL file, or 0 when no WAL exists.
    pub wal_bytes: u64,
    pub warnings: Vec<String>,
}

/// One displayed part. `text` preserves complete content; the UI controls disclosure.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PartView {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
    pub truncated: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessageView {
    pub id: String,
    pub role: String,
    pub created: Option<i64>,
    pub parts: Vec<PartView>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRead {
    pub id: String,
    pub title: String,
    pub messages: Vec<MessageView>,
    pub truncated: bool,
    pub warnings: Vec<String>,
}

/// What a confirmed deletion would remove. `blocked_reason` is set by the command layer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeletePreview {
    pub session_ids: Vec<String>,
    pub titles: Vec<String>,
    pub size_bytes: u64,
    pub blocked_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
    pub deleted_session_ids: Vec<String>,
    pub backup: String,
    pub message: String,
}

/// Session ordering accepted by [`crate::chat::list_sessions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Size,
    Updated,
    Created,
}

impl Sort {
    /// Parses the IPC value. Unknown values are rejected instead of silently defaulting.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "size" => Some(Self::Size),
            "updated" => Some(Self::Updated),
            "created" => Some(Self::Created),
            _ => None,
        }
    }

    pub(crate) fn column(self) -> &'static str {
        match self {
            Self::Size => "size_bytes",
            Self::Updated => "s.updated",
            Self::Created => "s.created",
        }
    }
}
