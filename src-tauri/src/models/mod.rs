//! Model catalogue read from the installed `OpenCode` CLI.

mod cli;
mod parse;
mod process;

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

use process::Outcome;

pub use parse::{parse_output, sanitize_diagnostic};

const CACHED_TIMEOUT: Duration = Duration::from_secs(60);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_OUTPUT_MIB: u64 = 1;

/// One verified model ID split at its first slash.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ModelEntry {
    pub id: String,
    pub provider: String,
    pub model: String,
}

/// The verified catalogue plus the CLI working directory used for the run.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ModelCatalog {
    pub models: Vec<ModelEntry>,
    pub directory: String,
}

/// Paths needed to run the CLI: the project root as cwd and the resolved `OpenCode` target file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelContext {
    pub root: PathBuf,
    pub config: PathBuf,
}

/// Runs the installed `OpenCode` CLI and returns only verified model IDs.
///
/// `refresh` is the only switch: it adds `--refresh`, otherwise the cached list is read. The child
/// inherits credentials and plugin configuration; `OPENCODE_CONFIG` points at the selected target.
///
/// # Errors
///
/// Returns [`AppError::CliUnavailable`] when no executable is found, [`AppError::CliFailed`] when
/// the process cannot start, fails, or its output cannot be collected, [`AppError::CliTimeout`]
/// when the bounded run is killed, and [`AppError::ModelOutputInvalid`] when stdout cannot be
/// trusted.
pub fn list(context: &ModelContext, refresh: bool) -> Result<ModelCatalog, AppError> {
    let executable = cli::resolve_executable()?;
    let args: &[&str] = if refresh {
        &["models", "--refresh"]
    } else {
        &["models"]
    };
    let timeout = if refresh {
        REFRESH_TIMEOUT
    } else {
        CACHED_TIMEOUT
    };
    let output = process::run(&executable, &context.root, &context.config, args, timeout)?;
    match output.outcome {
        Outcome::TimedOut => {
            return Err(AppError::CliTimeout {
                seconds: timeout.as_secs(),
            });
        }
        Outcome::Overflowed => {
            return Err(AppError::ModelOutputInvalid {
                reason: format!("命令输出超过 {MAX_OUTPUT_MIB} MiB 上限。"),
            });
        }
        Outcome::Incomplete => {
            return Err(AppError::CliFailed {
                reason: "读取 OpenCode 命令输出超时。".to_string(),
            });
        }
        Outcome::Completed { success: false } => {
            let code = output
                .code
                .map_or_else(|| "未知".to_string(), |value| value.to_string());
            let detail = sanitize_diagnostic(&output.stderr);
            let reason = if detail.is_empty() {
                format!("命令以退出码 {code} 结束。")
            } else {
                format!("命令以退出码 {code} 结束：{detail}")
            };
            return Err(AppError::CliFailed { reason });
        }
        Outcome::Completed { success: true } => {}
    }
    Ok(ModelCatalog {
        models: parse_output(&output.stdout)?,
        directory: context.root.display().to_string(),
    })
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
