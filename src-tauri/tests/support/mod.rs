#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use switcher_core::{Engine, Preset, Target};

/// Isolated project-local test directory that removes itself when dropped.
pub struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    pub fn new(name: &str) -> Self {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-work");
        fs::create_dir_all(&base).expect("create test-work directory");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let path = base.join(format!("{name}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path).expect("create test root");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn engine(&self) -> Engine {
        Engine::new(self.path.clone())
    }

    pub fn write(&self, relative: &str, content: &str) -> PathBuf {
        self.write_bytes(relative, content.as_bytes())
    }

    pub fn write_bytes(&self, relative: &str, content: &[u8]) -> PathBuf {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent directory");
        }
        fs::write(&path, content).expect("write test file");
        path
    }

    pub fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path.join(relative)).expect("read test file")
    }

    pub fn exists(&self, relative: &str) -> bool {
        self.path.join(relative).exists()
    }

    pub fn settings_path(&self) -> PathBuf {
        self.path.join("switcher.settings.json")
    }

    pub fn seed_settings(&self, opencode: &str, omo: &str, theme: &str) {
        let value = json!({"targets": {"opencode": opencode, "omo": omo}, "theme": theme});
        self.write("switcher.settings.json", &value.to_string());
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn preset(target: Target, name: &str, content: &str) -> Preset {
    Preset {
        id: String::new(),
        name: name.to_string(),
        description: String::new(),
        target,
        content: content.to_string(),
    }
}
