//! Process supervisor: spawns child processes safely without a shell.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};

pub type SupervisorResult<T> = Result<T, String>;

const MAX_STARTUP_DIAGNOSTICS: usize = 24;
const MAX_DIAGNOSTIC_LINE_CHARS: usize = 480;
#[cfg(windows)]
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct ManagedChild {
    process: Child,
    startup_diagnostics: Arc<Mutex<VecDeque<String>>>,
    capture_startup_diagnostics: Arc<AtomicBool>,
    stderr_task: Option<JoinHandle<()>>,
}

pub struct ProcessSupervisor {
    children: Arc<Mutex<HashMap<String, ManagedChild>>>,
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self {
            children: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn spawn(
        &self,
        label: &str,
        executable: PathBuf,
        args: Vec<OsString>,
        env_allowlist: &[String],
        working_dir: Option<PathBuf>,
    ) -> SupervisorResult<u32> {
        {
            let mut children = self.children.lock().await;
            let running = match children.get_mut(label) {
                Some(child) => child
                    .process
                    .try_wait()
                    .map_err(|e| format!("Failed to inspect {label} process: {e}"))?
                    .is_none(),
                None => false,
            };
            if running {
                return Err(format!("A process with label {label} is already running"));
            }
            children.remove(label);
        }

        let mut command = Command::new(&executable);
        command
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);

        if let Some(working_dir) = working_dir {
            command.current_dir(working_dir);
        }

        let env_vars: Vec<(String, String)> = std::env::vars()
            .filter(|(key, _)| {
                env_allowlist
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(key))
            })
            .collect();
        command.env_clear();
        command.envs(env_vars);

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn {label} process: {e}"))?;
        let pid = child
            .id()
            .ok_or_else(|| format!("The {label} process did not provide a process id"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| format!("The {label} process did not expose its diagnostic stream"))?;
        let startup_diagnostics = Arc::new(Mutex::new(VecDeque::new()));
        let capture_startup_diagnostics = Arc::new(AtomicBool::new(true));
        let diagnostics = startup_diagnostics.clone();
        let capture = capture_startup_diagnostics.clone();
        let stderr_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if capture.load(Ordering::Relaxed) && is_startup_diagnostic(&line) {
                            let line = strip_ansi_escape_codes(&line)
                                .chars()
                                .take(MAX_DIAGNOSTIC_LINE_CHARS)
                                .collect();
                            let mut diagnostics = diagnostics.lock().await;
                            if diagnostics.len() == MAX_STARTUP_DIAGNOSTICS {
                                diagnostics.pop_front();
                            }
                            diagnostics.push_back(line);
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        if capture.load(Ordering::Relaxed) {
                            let mut diagnostics = diagnostics.lock().await;
                            if diagnostics.len() == MAX_STARTUP_DIAGNOSTICS {
                                diagnostics.pop_front();
                            }
                            diagnostics
                                .push_back(format!("Failed to read runtime diagnostics: {error}"));
                        }
                        break;
                    }
                }
            }
        });

        self.children.lock().await.insert(
            label.to_string(),
            ManagedChild {
                process: child,
                startup_diagnostics,
                capture_startup_diagnostics,
                stderr_task: Some(stderr_task),
            },
        );
        Ok(pid)
    }

    pub async fn is_running(&self, label: &str) -> SupervisorResult<bool> {
        let mut children = self.children.lock().await;
        let exit_status = match children.get_mut(label) {
            Some(child) => child
                .process
                .try_wait()
                .map_err(|e| format!("Failed to inspect {label} process: {e}"))?,
            None => return Ok(false),
        };

        if exit_status.is_some() {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub async fn exit_status(&self, label: &str) -> SupervisorResult<Option<ExitStatus>> {
        let mut children = self.children.lock().await;
        let exit_status = match children.get_mut(label) {
            Some(child) => child
                .process
                .try_wait()
                .map_err(|e| format!("Failed to inspect {label} process: {e}"))?,
            None => return Ok(None),
        };
        Ok(exit_status)
    }

    pub async fn kill(&self, label: &str) -> SupervisorResult<()> {
        self.kill_with_diagnostics(label).await.map(|_| ())
    }

    pub async fn kill_with_diagnostics(&self, label: &str) -> SupervisorResult<Vec<String>> {
        let Some(mut child) = self.children.lock().await.remove(label) else {
            return Ok(Vec::new());
        };
        child
            .capture_startup_diagnostics
            .store(false, Ordering::Relaxed);

        if child
            .process
            .try_wait()
            .map_err(|e| format!("Failed to inspect {label} process: {e}"))?
            .is_none()
        {
            child
                .process
                .kill()
                .await
                .map_err(|e| format!("Failed to terminate {label} process: {e}"))?;
        }

        child
            .process
            .wait()
            .await
            .map_err(|e| format!("Failed to reap {label} process: {e}"))?;

        if let Some(stderr_task) = child.stderr_task.take() {
            timeout(Duration::from_secs(2), stderr_task)
                .await
                .map_err(|_| format!("Timed out while collecting {label} diagnostics"))?
                .map_err(|e| format!("Failed to collect {label} diagnostics: {e}"))?;
        }

        let diagnostics = child
            .startup_diagnostics
            .lock()
            .await
            .iter()
            .cloned()
            .collect();
        Ok(diagnostics)
    }

    pub async fn active_count(&self) -> usize {
        self.children.lock().await.len()
    }
}

fn is_startup_diagnostic(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    [
        "error",
        "failed",
        "fatal",
        "out of memory",
        "not enough memory",
        "unsupported",
        "cannot ",
        "invalid ",
        "unknown argument",
        "unable to",
    ]
    .iter()
    .any(|marker| line.contains(marker))
}

fn strip_ansi_escape_codes(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for character in chars.by_ref() {
                if ('@'..='~').contains(&character) {
                    break;
                }
            }
        } else {
            result.push(character);
        }
    }
    result
}

impl Default for ProcessSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_diagnostic_filter_only_keeps_errors() {
        assert!(is_startup_diagnostic(
            "common_init_from_params: failed to load model"
        ));
        assert!(is_startup_diagnostic(
            "ggml_backend_load: error loading backend"
        ));
        assert!(!is_startup_diagnostic(
            "llama_model_loader: loaded meta data with 42 key-value pairs"
        ));
    }

    #[test]
    fn test_startup_diagnostics_strip_terminal_colors() {
        assert_eq!(
            strip_ansi_escape_codes("\u{1b}[31mE model failed\u{1b}[0m"),
            "E model failed"
        );
    }

    fn where_executable() -> PathBuf {
        PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot must be set"))
            .join("System32")
            .join("where.exe")
    }

    #[tokio::test]
    async fn test_supervisor_starts_and_reaps_process() {
        let supervisor = ProcessSupervisor::new();
        let result = supervisor
            .spawn(
                "test",
                where_executable(),
                vec![OsString::from("llama-server.exe")],
                &[],
                None,
            )
            .await;

        assert!(result.is_ok());
        supervisor.kill("test").await.unwrap();
        assert_eq!(supervisor.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_supervisor_rejects_duplicate_label() {
        let supervisor = ProcessSupervisor::new();
        let executable = where_executable();
        supervisor
            .spawn(
                "test",
                executable.clone(),
                vec![OsString::from("llama-server.exe")],
                &[],
                None,
            )
            .await
            .unwrap();

        let result = supervisor
            .spawn(
                "test",
                executable,
                vec![OsString::from("llama-server.exe")],
                &[],
                None,
            )
            .await;

        assert!(result.is_err());
        supervisor.kill("test").await.unwrap();
    }
}
