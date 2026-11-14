//! Read-only chat history inspection and confirmed deletion for `OpenCode`'s SQLite database.

mod backup;
mod db;
mod delete;
mod descendants;
mod discover;
mod error;
mod json_text;
mod list;
mod parse;
mod process;
mod read;
mod schema;
mod schema_detect;
mod types;

pub use backup::backup_database;
pub use delete::{
    execute as execute_delete, execute_with_options as execute_delete_with_options,
    preview as preview_delete, DeleteOptions,
};
pub use discover::discover;
pub use error::{ChatError, ChatResult};
pub use list::list_sessions;
pub use process::opencode_running;
pub use read::read_session;
pub use types::{
    DeletePreview, DeleteReport, DiscoveryResult, MessageView, PartView, SessionList, SessionRead,
    SessionSummary, Sort,
};
