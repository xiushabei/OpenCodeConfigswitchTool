use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppError;
use crate::model::{
    Preset, PresetsFile, Settings, DATA_DIR, PRESETS_FILE, PRESET_FORMAT_VERSION, SETTINGS_FILE,
};
use crate::validate;

const TEMP_MARKER: &str = ".omo-switcher-tmp";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Reads and validates the project settings file.
///
/// # Errors
///
/// Returns [`AppError`] when the settings file cannot be read, cannot be parsed, or fails
/// validation.
pub fn read_settings(root: &Path) -> Result<Settings, AppError> {
    let path = root.join(SETTINGS_FILE);
    let raw = fs::read(&path).map_err(|err| AppError::io("读取设置文件", &path, err))?;
    let settings: Settings =
        serde_json::from_slice(&raw).map_err(|err| AppError::SettingsInvalid {
            reason: format!("无法解析设置文件：{err}"),
        })?;
    validate::validate_settings(root, &settings)?;
    Ok(settings)
}

/// Serializes and atomically replaces the project settings file.
///
/// # Errors
///
/// Returns [`AppError`] when the settings cannot be serialized or the file cannot be replaced.
pub fn write_settings(root: &Path, settings: &Settings) -> Result<(), AppError> {
    let path = root.join(SETTINGS_FILE);
    let mut json =
        serde_json::to_string_pretty(settings).map_err(|err| AppError::serialize(&err))?;
    json.push('\n');
    atomic_write(&path, json.as_bytes())
}

/// Preset storage plus the persisted first-import marker state.
pub struct PresetsStore {
    pub presets: Vec<Preset>,
    /// True when the one-time first-start import already completed or the file predates the
    /// marker.
    pub initialized: bool,
    /// Distinguishes a missing file from an existing file without the marker.
    pub existed: bool,
}

/// Reads the preset storage file, treating a missing file as uninitialized empty storage.
///
/// # Errors
///
/// Returns [`AppError`] when the file exists but cannot be read, cannot be parsed, or uses an
/// unsupported format version.
pub fn read_presets_store(root: &Path) -> Result<PresetsStore, AppError> {
    let path = root.join(PRESETS_FILE);
    let raw = match fs::read(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(PresetsStore {
                presets: Vec::new(),
                initialized: false,
                existed: false,
            })
        }
        Err(err) => return Err(AppError::io("读取预设存储", &path, err)),
    };
    let parsed: PresetsFile =
        serde_json::from_slice(&raw).map_err(|err| AppError::PresetStorageInvalid {
            reason: format!("无法解析预设存储：{err}"),
        })?;
    if parsed.format_version != PRESET_FORMAT_VERSION {
        return Err(AppError::PresetStorageInvalid {
            reason: format!("不支持的 formatVersion：{}", parsed.format_version),
        });
    }
    let initialized = parsed.is_initialized();
    Ok(PresetsStore {
        presets: parsed.presets,
        initialized,
        existed: true,
    })
}

/// Reads the preset storage file, treating a missing file as an empty preset list.
///
/// # Errors
///
/// Returns [`AppError`] when the file exists but cannot be read, cannot be parsed, or uses an
/// unsupported format version.
pub fn read_presets(root: &Path) -> Result<Vec<Preset>, AppError> {
    read_presets_store(root).map(|store| store.presets)
}

/// Serializes `presets` and the first-import marker, atomically replacing the preset storage file.
///
/// # Errors
///
/// Returns [`AppError`] when the data directory cannot be created, the presets cannot be
/// serialized, or the file cannot be replaced.
pub fn write_presets(root: &Path, presets: &[Preset], initialized: bool) -> Result<(), AppError> {
    let data_dir = root.join(DATA_DIR);
    fs::create_dir_all(&data_dir).map_err(|err| AppError::io("创建数据目录", &data_dir, err))?;
    let file = PresetsFile {
        format_version: PRESET_FORMAT_VERSION,
        presets: presets.to_vec(),
        first_import_completed: Some(initialized),
    };
    let mut json = serde_json::to_string_pretty(&file).map_err(|err| AppError::serialize(&err))?;
    json.push('\n');
    atomic_write(&root.join(PRESETS_FILE), json.as_bytes())
}

/// Replaces `path` atomically: write a sibling temp file, then rename over the destination.
///
/// # Errors
///
/// Returns [`AppError`] when the path has no parent directory, the parent is missing, or the
/// temp file cannot be written or renamed over the destination.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| AppError::TargetPathInvalid {
        reason: format!("路径没有父目录：{}", path.display()),
    })?;
    if !parent.is_dir() {
        return Err(AppError::io(
            "写入文件",
            path,
            io::Error::new(ErrorKind::NotFound, "父目录不存在"),
        ));
    }
    let temp = temp_path(parent, path.file_name());
    let write_result = write_new_file(&temp, bytes);
    if let Err(err) = write_result {
        return Err(AppError::io(
            "写入临时文件",
            &temp,
            cleanup_temp(&temp, err),
        ));
    }
    if let Err(err) = fs::rename(&temp, path) {
        return Err(AppError::io("替换文件", path, cleanup_temp(&temp, err)));
    }
    Ok(())
}

/// Creates `path` exclusively and writes `bytes`, without replacing an existing file.
///
/// # Errors
///
/// Returns the I/O error when the file already exists or cannot be written and synced.
pub fn try_create_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[must_use]
pub fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

/// Deletes a leftover temp file and folds any cleanup failure into the original error.
fn cleanup_temp(temp: &Path, error: io::Error) -> io::Error {
    match fs::remove_file(temp) {
        Ok(()) => error,
        Err(cleanup) => io::Error::new(
            error.kind(),
            format!("{error}；同时无法清理临时文件：{cleanup}"),
        ),
    }
}

fn write_new_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn temp_path(parent: &Path, name: Option<&OsStr>) -> PathBuf {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let stamp = timestamp_millis();
    let stem = name.map_or_else(
        || "file".to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    parent.join(format!("{TEMP_MARKER}-{stem}-{stamp}-{sequence}"))
}
