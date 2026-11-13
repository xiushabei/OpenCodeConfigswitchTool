use std::io::{ErrorKind, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::AppError;

use super::cli::Executable;

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const READER_GRACE: Duration = Duration::from_secs(2);
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// How a bounded child run ended. Failure flags are exclusive so callers cannot misread them.
#[derive(Clone, Copy)]
pub(super) enum Outcome {
    Completed { success: bool },
    TimedOut,
    Overflowed,
    Incomplete,
}

/// Completed child process data. Output buffers are capped; a capped stream reports `Overflowed`.
pub(super) struct CommandOutput {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub outcome: Outcome,
}

pub(super) fn run(
    executable: &Executable,
    cwd: &Path,
    config: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandOutput, AppError> {
    let mut command = build_command(executable, args);
    command
        .current_dir(cwd)
        .env("OPENCODE_CONFIG", config)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|err| AppError::CliFailed {
        reason: format!("无法启动 OpenCode 命令：{err}"),
    })?;
    let stdout_pipe = child.stdout.take().ok_or_else(|| pipe_error("标准输出"))?;
    let stderr_pipe = child.stderr.take().ok_or_else(|| pipe_error("标准错误"))?;

    let overflow = Arc::new(AtomicBool::new(false));
    let stdout_rx = read_capped(stdout_pipe, Arc::clone(&overflow));
    let stderr_rx = read_capped(stderr_pipe, Arc::clone(&overflow));

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::CliFailed {
                    reason: format!("等待 OpenCode 命令失败：{err}"),
                });
            }
        }
        if overflow.load(Ordering::Relaxed) {
            let _ = child.kill();
            break child.wait().map_err(|err| wait_error(&err))?;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            break child.wait().map_err(|err| wait_error(&err))?;
        }
        std::thread::sleep(POLL_INTERVAL);
    };

    let (stdout, stdout_ok) = receive(&stdout_rx);
    let (stderr, stderr_ok) = receive(&stderr_rx);
    let overflowed = overflow.load(Ordering::Relaxed);
    let outcome = if timed_out {
        Outcome::TimedOut
    } else if overflowed {
        Outcome::Overflowed
    } else if stdout_ok && stderr_ok {
        Outcome::Completed {
            success: status.success(),
        }
    } else {
        Outcome::Incomplete
    };
    Ok(CommandOutput {
        code: status.code(),
        stdout,
        stderr,
        outcome,
    })
}

fn build_command(executable: &Executable, args: &[&str]) -> Command {
    match executable {
        Executable::Direct(path) => {
            let mut command = Command::new(path);
            command.args(args);
            command
        }
        #[cfg(windows)]
        Executable::Script { comspec, script } => {
            use std::os::windows::process::CommandExt as _;
            let mut command = Command::new(comspec);
            command
                .arg("/D")
                .arg("/S")
                .arg("/C")
                .raw_arg(script_line(script, args));
            command
        }
    }
}

/// Builds one `/C` command string. Arguments are fixed literals, never user input.
#[cfg(windows)]
fn script_line(script: &Path, args: &[&str]) -> String {
    let mut line = format!("\"{}\"", script.display());
    for arg in args {
        line.push(' ');
        line.push_str(arg);
    }
    format!("\"{line}\"")
}

fn read_capped<R: Read + Send + 'static>(
    mut pipe: R,
    overflow: Arc<AtomicBool>,
) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut data = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            match pipe.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let room = MAX_OUTPUT_BYTES.saturating_sub(data.len());
                    if read > room {
                        overflow.store(true, Ordering::Relaxed);
                    }
                    data.extend_from_slice(&buffer[..read.min(room)]);
                }
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        let _ = sender.send(data);
    });
    receiver
}

fn receive(receiver: &Receiver<Vec<u8>>) -> (Vec<u8>, bool) {
    receiver
        .recv_timeout(READER_GRACE)
        .map_or_else(|_| (Vec::new(), false), |data| (data, true))
}

fn pipe_error(stream: &str) -> AppError {
    AppError::CliFailed {
        reason: format!("无法读取 OpenCode {stream}。"),
    }
}

fn wait_error(err: &std::io::Error) -> AppError {
    AppError::CliFailed {
        reason: format!("终止 OpenCode 命令失败：{err}"),
    }
}
