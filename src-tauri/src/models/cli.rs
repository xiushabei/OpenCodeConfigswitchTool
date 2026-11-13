use std::env;
use std::path::PathBuf;

use crate::error::AppError;

const EXE_NAME: &str = if cfg!(windows) {
    "opencode.exe"
} else {
    "opencode"
};

/// A resolved `OpenCode` entry point. Batch wrappers must run through the command interpreter.
pub(super) enum Executable {
    Direct(PathBuf),
    #[cfg(windows)]
    Script {
        comspec: PathBuf,
        script: PathBuf,
    },
}

pub(super) fn resolve_executable() -> Result<Executable, AppError> {
    let dirs: Vec<PathBuf> = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect())
        .unwrap_or_default();
    resolve_in(&dirs)
}

pub(super) fn resolve_in(dirs: &[PathBuf]) -> Result<Executable, AppError> {
    for dir in dirs {
        let candidate = dir.join(EXE_NAME);
        if candidate.is_file() {
            return Ok(Executable::Direct(candidate));
        }
    }
    #[cfg(windows)]
    if let Some(executable) = resolve_npm_script(dirs) {
        return Ok(executable);
    }
    Err(AppError::CliUnavailable {
        reason: if cfg!(windows) {
            "未在 PATH 中找到 opencode.exe 或 npm 安装的 opencode.cmd。".to_string()
        } else {
            "未在 PATH 中找到 opencode。".to_string()
        },
    })
}

/// npm installs a `.cmd` wrapper next to `node_modules/opencode-ai/bin/opencode.exe`. Prefer that
/// executable; fall back to the wrapper through `ComSpec` when the package layout differs.
#[cfg(windows)]
fn resolve_npm_script(dirs: &[PathBuf]) -> Option<Executable> {
    let mut fallback = None;
    for dir in dirs {
        let script = dir.join("opencode.cmd");
        if !script.is_file() {
            continue;
        }
        let npm_exe = dir
            .join("node_modules")
            .join("opencode-ai")
            .join("bin")
            .join("opencode.exe");
        if npm_exe.is_file() {
            return Some(Executable::Direct(npm_exe));
        }
        fallback.get_or_insert(script);
    }
    fallback.map(|script| Executable::Script {
        comspec: comspec(),
        script,
    })
}

#[cfg(windows)]
pub(super) fn comspec() -> PathBuf {
    if let Some(value) = env::var_os("ComSpec") {
        let path = PathBuf::from(value);
        if path.is_absolute() && path.is_file() {
            return path;
        }
    }
    let root =
        env::var_os("SystemRoot").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    let cmd = root.join("System32").join("cmd.exe");
    if cmd.is_file() {
        cmd
    } else {
        PathBuf::from("cmd.exe")
    }
}
