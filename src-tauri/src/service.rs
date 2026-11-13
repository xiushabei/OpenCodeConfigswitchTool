use std::path::Path;

use crate::error::AppError;
use crate::model::{Preset, Settings, Snapshot, Target, TargetState};
use crate::{storage, targets, validate};

/// Reads persisted settings and presets, then describes every target.
///
/// # Errors
///
/// Returns [`AppError`] when the settings or preset storage cannot be read or validated.
pub fn snapshot(root: &Path) -> Result<Snapshot, AppError> {
    let settings = storage::read_settings(root)?;
    let presets = storage::read_presets(root)?;
    Ok(build_snapshot(root, settings, presets))
}

#[must_use]
pub fn build_snapshot(root: &Path, settings: Settings, presets: Vec<Preset>) -> Snapshot {
    let mut states: Vec<TargetState> = Target::ALL
        .iter()
        .map(|target| targets::read_target_state(root, &settings, *target))
        .collect();
    targets::mark_active(&mut states, &presets);
    Snapshot {
        settings,
        presets,
        targets: states,
        root: root.display().to_string(),
        startup_warnings: Vec::new(),
        can_restore_startup: false,
    }
}

/// Creates a preset when `id` is empty, otherwise replaces the existing preset with that id.
///
/// # Errors
///
/// Returns [`AppError`] when the preset is invalid, the id is unknown, or the storage write or
/// refresh fails.
pub fn save_preset(root: &Path, preset: Preset) -> Result<Snapshot, AppError> {
    validate::validate_preset_input(&preset)?;
    let store = storage::read_presets_store(root)?;
    let initialized = store.initialized || !store.existed;
    let mut presets = store.presets;

    let mut preset = preset;
    let id = preset.id.trim().to_string();
    if id.is_empty() {
        preset.id = new_preset_id(&presets);
        presets.push(preset);
    } else {
        preset.id.clone_from(&id);
        let existing = presets
            .iter_mut()
            .find(|candidate| candidate.id == id)
            .ok_or(AppError::PresetNotFound { id })?;
        *existing = preset;
    }

    storage::write_presets(root, &presets, initialized)?;
    snapshot(root)
}

/// Removes the preset with `id` and returns the refreshed snapshot.
///
/// # Errors
///
/// Returns [`AppError::PresetNotFound`] when no preset has that id, or the storage error when the
/// presets cannot be read, written, or refreshed.
pub fn delete_preset(root: &Path, id: &str) -> Result<Snapshot, AppError> {
    let id = id.trim();
    let store = storage::read_presets_store(root)?;
    let initialized = store.initialized || !store.existed;
    let mut presets = store.presets;
    let before = presets.len();
    presets.retain(|preset| preset.id != id);
    if presets.len() == before {
        return Err(AppError::PresetNotFound { id: id.to_string() });
    }
    storage::write_presets(root, &presets, initialized)?;
    snapshot(root)
}

/// Validates and stores `settings`, then describes every target from the stored values.
///
/// # Errors
///
/// Returns [`AppError`] when the settings are invalid or cannot be written, or the presets cannot
/// be read.
pub fn save_settings(root: &Path, settings: Settings) -> Result<Snapshot, AppError> {
    validate::validate_settings(root, &settings)?;
    storage::write_settings(root, &settings)?;
    let presets = storage::read_presets(root)?;
    Ok(build_snapshot(root, settings, presets))
}

fn new_preset_id(presets: &[Preset]) -> String {
    let stamp = storage::timestamp_millis();
    let mut candidate = format!("preset-{stamp}");
    let mut suffix = 1;
    while presets.iter().any(|preset| preset.id == candidate) {
        candidate = format!("preset-{stamp}-{suffix}");
        suffix += 1;
    }
    candidate
}
