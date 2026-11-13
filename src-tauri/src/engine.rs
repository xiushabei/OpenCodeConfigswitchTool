use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::error::AppError;
use crate::model::{ApplyResult, Preset, Settings, Snapshot};
use crate::startup::StartupCapture;
use crate::{apply, config_import, export, import, restore, service, startup};

/// Owns the project root and serializes every command that reads or writes owned files.
pub struct Engine {
    root: PathBuf,
    lock: Mutex<()>,
    startup: OnceLock<StartupCapture>,
}

impl Engine {
    /// Creates an engine rooted at `root` with no startup baseline yet captured.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            lock: Mutex::new(()),
            startup: OnceLock::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn guard(&self) -> Result<MutexGuard<'_, ()>, AppError> {
        self.lock.lock().map_err(|_| AppError::LockPoisoned)
    }

    /// Captures the launch baseline and imports valid current configurations exactly once.
    ///
    /// Later calls never replace the baseline. Failures are recorded as snapshot warnings so the
    /// window can still open.
    pub fn initialize_startup(&self) {
        let Ok(_guard) = self.guard() else {
            return;
        };
        let _ = self.startup.get_or_init(|| startup::capture(&self.root));
    }

    fn enrich(&self, snapshot: &mut Snapshot) {
        snapshot.startup_warnings = self
            .startup
            .get()
            .map(|capture| capture.warnings().to_vec())
            .unwrap_or_default();
        snapshot.can_restore_startup = self.can_restore_startup();
    }

    /// Reports whether a complete launch baseline exists for [`Self::restore_startup`].
    #[must_use]
    pub fn can_restore_startup(&self) -> bool {
        self.startup.get().is_some_and(StartupCapture::can_restore)
    }

    /// Reads the current settings, presets, and target states.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked, or the settings or
    /// preset storage error when either cannot be read.
    pub fn snapshot(&self) -> Result<Snapshot, AppError> {
        let _guard = self.guard()?;
        let mut snapshot = service::snapshot(&self.root)?;
        self.enrich(&mut snapshot);
        Ok(snapshot)
    }

    /// Stores `preset`, creating it when its id is empty and replacing it otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked, or the validation,
    /// lookup, or storage error when the preset cannot be saved.
    pub fn save_preset(&self, preset: Preset) -> Result<Snapshot, AppError> {
        let _guard = self.guard()?;
        let mut snapshot = service::save_preset(&self.root, preset)?;
        self.enrich(&mut snapshot);
        Ok(snapshot)
    }

    /// Removes the preset with `id`.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked, or the lookup or
    /// storage error when the preset cannot be deleted.
    pub fn delete_preset(&self, id: &str) -> Result<Snapshot, AppError> {
        let _guard = self.guard()?;
        let mut snapshot = service::delete_preset(&self.root, id)?;
        self.enrich(&mut snapshot);
        Ok(snapshot)
    }

    /// Validates and stores `settings`, then returns the resulting snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked, or the validation,
    /// storage, or snapshot error when the settings cannot be saved.
    pub fn save_settings(&self, settings: Settings) -> Result<Snapshot, AppError> {
        let _guard = self.guard()?;
        let mut snapshot = service::save_settings(&self.root, settings)?;
        self.enrich(&mut snapshot);
        Ok(snapshot)
    }

    /// Applies the preset with `id` to its target file.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked, or the apply error
    /// when validation, backup, or the target write fails.
    pub fn apply_preset(&self, id: &str) -> Result<ApplyResult, AppError> {
        let _guard = self.guard()?;
        let mut result = apply::apply_preset(&self.root, id)?;
        self.enrich(&mut result.snapshot);
        Ok(result)
    }

    /// Restores both launch targets to their exact startup bytes.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked,
    /// [`AppError::StartupBaselineMissing`] when startup did not capture a baseline,
    /// [`AppError::StartupRestoreBlocked`] when the baseline is incomplete, or the preflight,
    /// backup, restore, or refresh error when the restore cannot complete.
    pub fn restore_startup(&self) -> Result<ApplyResult, AppError> {
        let _guard = self.guard()?;
        let capture = self.startup.get().ok_or(AppError::StartupBaselineMissing)?;
        let baseline = capture
            .baseline()
            .ok_or_else(|| AppError::StartupRestoreBlocked {
                reason: capture.unavailable_reason(),
            })?;
        let mut result = restore::restore_startup(&self.root, baseline)?;
        self.enrich(&mut result.snapshot);
        Ok(result)
    }

    /// Reads a dropped `.json`/`.jsonc` import file as UTF-8 text.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked,
    /// [`AppError::ImportInvalid`] when the extension or content is rejected, or
    /// [`AppError::Io`] when the file cannot be read.
    pub fn read_import_file(&self, path: &str) -> Result<String, AppError> {
        let _guard = self.guard()?;
        import::read_import_text(path)
    }

    /// Normalizes pasted or browser-read JSON/JSONC object content to strict JSON.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked, or
    /// [`AppError::ImportInvalid`] when the content is not a JSON/JSONC object.
    pub fn normalize_import_content(&self, content: &str) -> Result<String, AppError> {
        let _guard = self.guard()?;
        import::normalize_content(content)
    }

    /// Reads the current settings targets and appends them as presets on every call.
    ///
    /// Existing presets are never replaced, even when their content matches.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked, the settings or
    /// preset storage error when either cannot be read or written, or the refresh error when the
    /// resulting snapshot cannot be built.
    pub fn import_current_configs(&self) -> Result<Snapshot, AppError> {
        let _guard = self.guard()?;
        let outcome = config_import::import_current(&self.root)?;
        let mut snapshot = service::snapshot(&self.root)?;
        self.enrich(&mut snapshot);
        snapshot.startup_warnings.extend(outcome.warnings);
        Ok(snapshot)
    }

    /// Exports the preset with `id` and returns its project-relative path.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::LockPoisoned`] when a previous command panicked, or the lookup or
    /// storage error when the export file cannot be written.
    pub fn export_preset(&self, id: &str) -> Result<String, AppError> {
        let _guard = self.guard()?;
        export::export_preset(&self.root, id)
    }
}
