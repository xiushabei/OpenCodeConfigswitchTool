use std::path::{Component, Path, PathBuf};

use crate::error::AppError;
use crate::model::SETTINGS_FILE;

/// Finds the project root by walking executable ancestors first, then cwd ancestors.
#[must_use]
pub fn discover_root(exe: Option<&Path>, cwd: &Path) -> Option<PathBuf> {
    if let Some(exe) = exe {
        let start = exe.parent().unwrap_or(exe);
        if let Some(root) = find_ancestor(start) {
            return Some(root);
        }
    }
    let cwd = if cwd.is_absolute() {
        cwd.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(cwd)
    };
    find_ancestor(&cwd)
}

/// Locates the project root from the running executable or the current directory.
///
/// # Errors
///
/// Returns [`AppError`] when the current directory cannot be read or no ancestor contains
/// `switcher.settings.json`.
pub fn discover_root_from_env() -> Result<PathBuf, AppError> {
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir()
        .map_err(|err| AppError::io("读取当前目录", PathBuf::from("."), err))?;
    discover_root(exe.as_deref(), &cwd).ok_or(AppError::RootNotFound)
}

fn find_ancestor(start: &Path) -> Option<PathBuf> {
    let start = normalize(start);
    start
        .ancestors()
        .find(|dir| dir.join(SETTINGS_FILE).is_file())
        .map(Path::to_path_buf)
}

/// Resolves `.` and `..` lexically without touching the filesystem.
#[must_use]
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    let absolute = path.is_absolute();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() && !absolute {
                    out.push("..");
                }
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn components_lower(path: &Path) -> Vec<String> {
    normalize(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

#[must_use]
pub fn same_path(left: &Path, right: &Path) -> bool {
    components_lower(left) == components_lower(right)
}

#[must_use]
pub fn is_within(child: &Path, parent: &Path) -> bool {
    let child = components_lower(child);
    let parent = components_lower(parent);
    child.len() >= parent.len() && child[..parent.len()] == parent[..]
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

/// Resolves a configured target path against the project root and the current profile.
///
/// # Errors
///
/// Returns [`AppError::TargetPathInvalid`] when `raw` is empty, points at a directory, cannot be
/// expanded, or uses a Windows rewriting or reserved name.
pub fn resolve_target(root: &Path, raw: &str) -> Result<PathBuf, AppError> {
    resolve_target_in(root, raw, home_dir().as_deref())
}

/// Resolves a configured target path. `home` is injectable so tests can avoid the real profile.
///
/// # Errors
///
/// Returns [`AppError::TargetPathInvalid`] when `raw` is empty, points at a directory, cannot be
/// expanded, or uses a Windows rewriting or reserved name.
pub fn resolve_target_in(root: &Path, raw: &str, home: Option<&Path>) -> Result<PathBuf, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::TargetPathInvalid {
            reason: "目标路径不能为空。".to_string(),
        });
    }
    if trimmed.ends_with('/') || trimmed.ends_with('\\') {
        return Err(AppError::TargetPathInvalid {
            reason: format!("目标路径不能指向目录：{trimmed}"),
        });
    }

    let expanded = if trimmed == "~" {
        return Err(AppError::TargetPathInvalid {
            reason: "目标路径不能是用户主目录。".to_string(),
        });
    } else if trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
        let rest = trimmed[1..].trim_start_matches(['/', '\\']);
        let home = home.ok_or_else(|| AppError::TargetPathInvalid {
            reason: "无法确定用户主目录，无法展开 ~。".to_string(),
        })?;
        home.join(rest)
    } else {
        PathBuf::from(trimmed)
    };

    let resolved = if expanded.is_absolute() {
        expanded
    } else {
        root.join(expanded)
    };
    let normalized = normalize(&resolved);
    if normalized.file_name().is_none() {
        return Err(AppError::TargetPathInvalid {
            reason: format!("目标路径缺少文件名：{raw}"),
        });
    }
    reject_windows_alias_components(&normalized)?;
    Ok(normalized)
}

/// Rejects names Windows silently rewrites (trailing dot/space) or reserves (devices),
/// because such paths could alias an application-owned file.
fn reject_windows_alias_components(path: &Path) -> Result<(), AppError> {
    for component in path.components() {
        if let Component::Normal(part) = component {
            let text = part.to_string_lossy();
            if text.ends_with('.') || text.ends_with(' ') {
                return Err(AppError::TargetPathInvalid {
                    reason: format!("目标路径包含 Windows 会改写的名称：{text}"),
                });
            }
            if is_reserved_device(&text) {
                return Err(AppError::TargetPathInvalid {
                    reason: format!("目标路径使用了 Windows 保留设备名：{text}"),
                });
            }
        }
    }
    Ok(())
}

fn is_reserved_device(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    let numbered = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"));
    numbered.is_some_and(|suffix| {
        suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit() && suffix != "0"
    })
}
