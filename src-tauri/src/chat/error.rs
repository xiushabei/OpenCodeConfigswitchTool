use std::fmt;

/// Errors returned by the chat database feature. Messages are user-facing.
#[derive(Debug)]
pub enum ChatError {
    NotFound {
        path: String,
    },
    InvalidPath {
        reason: String,
    },
    Open {
        path: String,
        reason: String,
    },
    Query {
        reason: String,
    },
    Schema {
        reason: String,
    },
    Backup {
        reason: String,
    },
    Blocked {
        reason: String,
    },
    Confirmation {
        expected: Vec<String>,
        confirmed: Vec<String>,
    },
    SessionNotFound {
        id: String,
    },
    Limit {
        reason: String,
    },
}

pub type ChatResult<T> = Result<T, ChatError>;

impl ChatError {
    #[must_use]
    pub fn query(error: &rusqlite::Error) -> Self {
        Self::Query {
            reason: error.to_string(),
        }
    }

    pub fn schema(reason: impl Into<String>) -> Self {
        Self::Schema {
            reason: reason.into(),
        }
    }

    pub fn backup(reason: impl Into<String>) -> Self {
        Self::Backup {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ChatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "聊天数据库不存在：{path}"),
            Self::InvalidPath { reason } => write!(f, "聊天数据库路径无效：{reason}"),
            Self::Open { path, reason } => write!(f, "无法只读打开聊天数据库（{path}）：{reason}"),
            Self::Query { reason } => write!(f, "读取聊天数据库失败：{reason}"),
            Self::Schema { reason } => write!(f, "无法识别的聊天数据库结构：{reason}"),
            Self::Backup { reason } => write!(f, "聊天数据库备份失败：{reason}"),
            Self::Blocked { reason } => write!(f, "已阻止删除聊天记录：{reason}"),
            Self::Confirmation {
                expected,
                confirmed,
            } => write!(
                f,
                "待删除会话与确认列表不一致（应删除 {} 个，已确认 {} 个），请重新预览后确认。",
                expected.len(),
                confirmed.len()
            ),
            Self::SessionNotFound { id } => write!(f, "会话不存在：{id}"),
            Self::Limit { reason } => write!(f, "聊天数据读取已中止：{reason}"),
        }
    }
}

impl std::error::Error for ChatError {}
