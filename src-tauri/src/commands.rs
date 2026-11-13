use tauri::State;

use crate::engine::Engine;
use crate::model::{ApplyResult, Preset, Settings, Snapshot};

/// Returns the current settings, presets, and target states.
///
/// # Errors
///
/// Returns a user-facing message when project settings or preset storage cannot be read.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn snapshot(engine: State<'_, Engine>) -> Result<Snapshot, String> {
    engine.snapshot().map_err(|err| err.to_string())
}

/// Creates a preset, or replaces the preset whose `id` is already set.
///
/// # Errors
///
/// Returns a user-facing message when the preset is invalid, its id is unknown, or the preset
/// storage cannot be written.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn save_preset(preset: Preset, engine: State<'_, Engine>) -> Result<Snapshot, String> {
    engine.save_preset(preset).map_err(|err| err.to_string())
}

/// Deletes the preset with `id`.
///
/// # Errors
///
/// Returns a user-facing message when the preset id is unknown or the preset storage cannot be
/// written.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn delete_preset(id: &str, engine: State<'_, Engine>) -> Result<Snapshot, String> {
    engine.delete_preset(id).map_err(|err| err.to_string())
}

/// Persists `settings` and returns a refreshed snapshot.
///
/// # Errors
///
/// Returns a user-facing message when the settings are invalid or cannot be written.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn save_settings(settings: Settings, engine: State<'_, Engine>) -> Result<Snapshot, String> {
    engine
        .save_settings(settings)
        .map_err(|err| err.to_string())
}

/// Applies the preset with `id` to its target file.
///
/// # Errors
///
/// Returns a user-facing message when the preset is unknown or invalid, the target path is
/// rejected, or the backup or target write fails.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn apply_preset(id: &str, engine: State<'_, Engine>) -> Result<ApplyResult, String> {
    engine.apply_preset(id).map_err(|err| err.to_string())
}

/// Restores both targets to their exact bytes captured at application startup.
///
/// # Errors
///
/// Returns a user-facing message when no complete startup baseline exists, the current targets
/// cannot be backed up, a restore fails, or the snapshot cannot be refreshed.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn restore_startup(engine: State<'_, Engine>) -> Result<ApplyResult, String> {
    engine.restore_startup().map_err(|err| err.to_string())
}

/// Reads a dropped `.json`/`.jsonc` file and returns strict JSON object text.
///
/// # Errors
///
/// Returns a user-facing message when the path is not a `.json`/`.jsonc` file, the content is not
/// UTF-8 JSON/JSONC object text, or the file cannot be read.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn read_import_file(path: String, engine: State<'_, Engine>) -> Result<String, String> {
    engine
        .read_import_file(&path)
        .map_err(|err| err.to_string())
}

/// Normalizes JSON/JSONC object content from an HTML file input or browser drop.
///
/// # Errors
///
/// Returns a user-facing message when the content is not a JSON/JSONC object.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn normalize_import_content(
    content: String,
    engine: State<'_, Engine>,
) -> Result<String, String> {
    engine
        .normalize_import_content(&content)
        .map_err(|err| err.to_string())
}

/// Imports the current settings targets as new presets on every call.
///
/// # Errors
///
/// Returns a user-facing message when settings or preset storage cannot be read or written, or
/// the refreshed snapshot cannot be built. Per-target read or parse problems are reported in
/// `startupWarnings` while the remaining target still imports.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn import_current_configs(engine: State<'_, Engine>) -> Result<Snapshot, String> {
    engine
        .import_current_configs()
        .map_err(|err| err.to_string())
}

/// Exports the preset with `id` into the project `exports/` directory.
///
/// # Errors
///
/// Returns a user-facing message when the preset is unknown or the export file cannot be written.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri resolves arguments via CommandArg, which is implemented for owned State"
)]
pub fn export_preset(id: &str, engine: State<'_, Engine>) -> Result<String, String> {
    engine.export_preset(id).map_err(|err| err.to_string())
}
