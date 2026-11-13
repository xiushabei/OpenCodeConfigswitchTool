use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::cli::{comspec, resolve_in, Executable};
use super::process::{run, Outcome};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
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

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn resolves_npm_installed_executable_before_wrapper() {
    let root = TempDir::new("cli-resolve");
    let bin = root.path().join("node_modules/opencode-ai/bin");
    fs::create_dir_all(&bin).expect("create npm layout");
    fs::write(root.path().join("opencode.cmd"), "@echo off\r\n").expect("write wrapper");
    let executable = bin.join("opencode.exe");
    fs::write(&executable, b"fake exe").expect("write npm executable");

    let resolved = resolve_in(&[root.path().to_path_buf()]).expect("resolve npm executable");
    match resolved {
        Executable::Direct(path) => assert_eq!(path, executable),
        #[cfg(windows)]
        Executable::Script { .. } => panic!("expected direct executable"),
    }
}

#[cfg(windows)]
#[test]
fn falls_back_to_comspec_for_npm_wrapper() {
    let root = TempDir::new("cli-wrapper");
    let script = root.path().join("opencode.cmd");
    fs::write(&script, "@echo off\r\n").expect("write wrapper");

    let resolved = resolve_in(&[root.path().to_path_buf()]).expect("resolve wrapper");
    match resolved {
        Executable::Script {
            comspec,
            script: resolved,
        } => {
            assert_eq!(resolved, script);
            assert_eq!(
                comspec.file_name().and_then(|name| name.to_str()),
                Some("cmd.exe")
            );
        }
        Executable::Direct(path) => panic!("expected wrapper, got {}", path.display()),
    }
}

#[cfg(windows)]
#[test]
fn runs_wrapper_with_fixed_args_environment_and_cwd() {
    let root = TempDir::new("cli-run");
    let script = root.path().join("opencode.cmd");
    fs::write(
        &script,
        "@echo off\r\necho cwd=%CD%\r\necho config=%OPENCODE_CONFIG%\r\necho args=%1 %2\r\necho alpha/one\r\necho beta/two/three\r\n",
    )
    .expect("write wrapper");
    let config = root.path().join("opencode.json");
    fs::write(&config, "{}").expect("write config");

    let spec = Executable::Script {
        comspec: comspec(),
        script,
    };
    let output = run(
        &spec,
        root.path(),
        &config,
        &["models", "--refresh"],
        Duration::from_secs(30),
    )
    .expect("run wrapper");

    assert!(
        matches!(output.outcome, Outcome::Completed { success: true }),
        "wrapper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("utf-8 output");
    let lower = text.to_lowercase();
    assert!(lower.contains(&format!(
        "cwd={}",
        root.path().display().to_string().to_lowercase()
    )));
    assert!(lower.contains(&format!(
        "config={}",
        config.display().to_string().to_lowercase()
    )));
    assert!(text.contains("args=models --refresh"));
    assert!(text.contains("alpha/one"));
    assert!(text.contains("beta/two/three"));
}
