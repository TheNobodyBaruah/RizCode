//! The compile + execute pipeline.
//!
//! ```text
//!   source (memory)
//!        │  write
//!        ▼
//!   throwaway workspace/<uuid>/main.{rs,py,js}
//!        │  rustc --edition 2021 -o prog main.rs      (Rust only)
//!        ▼
//!   program + stdin  ──spawn──▶  capture stdout/stderr, enforce limits
//! ```

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::language::Language;
use crate::limits::{Limits, COMPILE_TIMEOUT};
use crate::platform::Sandbox;
use crate::runtime;
use crate::workspace::Workspace;

/// The outcome of a compilation or execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// The program ran to completion (exit code may still be non-zero).
    Completed,
    /// The Rust source could not be compiled.
    CompileError,
    /// The program exceeded the wall-clock timeout and was killed.
    TimedOut,
    /// The program could not be launched (missing toolchain, unusable workspace, …).
    LaunchError,
    /// The process could not be spawned.
    SpawnError,
}

/// Everything needed to execute a piece of code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunRequest {
    /// The language the source is written in.
    pub language: Language,
    /// The source code, held in memory — it is never written to a permanent location.
    pub stdin: String,
    /// The source code to execute.
    pub code: String,
}

impl RunRequest {
    /// Creates a request without stdin input.
    pub fn new(language: Language, code: impl Into<String>) -> Self {
        Self {
            language,
            code: code.into(),
            stdin: String::new(),
        }
    }

    /// Attaches stdin input.
    pub fn with_stdin(mut self, stdin: impl Into<String>) -> Self {
        self.stdin = stdin.into();
        self
    }
}

/// The result of a run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunOutcome {
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// The exit code, or `None` when the process was killed by a signal.
    pub exit_code: Option<i32>,
    /// How long the execution took, in milliseconds.
    pub duration_ms: u64,
    /// The overall status.
    pub status: RunStatus,
    /// `true` when either stream exceeded the output limit and was cut short.
    pub truncated: bool,
}

impl RunOutcome {
    /// Builds a failure outcome that carries a message on stderr.
    fn failure(status: RunStatus, message: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            stdout: String::new(),
            stderr: message.into(),
            exit_code: None,
            duration_ms,
            status,
            truncated: false,
        }
    }
}

/// The program that is going to be executed.
enum Program {
    /// A native binary produced by the Rust compiler.
    Native { path: PathBuf },
    /// A script interpreted by an external interpreter.
    Script {
        program: std::ffi::OsString,
        run_args: Vec<std::ffi::OsString>,
        script: PathBuf,
    },
}

/// Executes code inside a throwaway, resource-limited workspace.
pub struct Runner {
    base_dir: PathBuf,
    limits: Limits,
}

impl Runner {
    /// Creates a runner that creates its workspaces inside `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            limits: Limits::default(),
        }
    }

    /// Overrides the resource limits.
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// The limits this runner applies.
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// The directory the throwaway workspaces are created in.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Compiles (if needed) and executes the request, capturing both output streams.
    pub fn run(&self, request: &RunRequest) -> RunOutcome {
        let started = Instant::now();

        let workspace = match Workspace::create(&self.base_dir) {
            Ok(workspace) => workspace,
            Err(err) => {
                return RunOutcome::failure(
                    RunStatus::LaunchError,
                    format!("could not create the workspace directory: {err}"),
                    started.elapsed().as_millis() as u64,
                )
            }
        };

        let source = workspace.file(request.language.file_name());
        if let Err(err) = std::fs::write(&source, &request.code) {
            return RunOutcome::failure(
                RunStatus::LaunchError,
                format!("could not write the source file: {err}"),
                started.elapsed().as_millis() as u64,
            );
        }

        let program = match request.language {
            Language::Rust => {
                let binary = workspace.file(if cfg!(windows) { "prog.exe" } else { "prog" });
                match self.compile_rust(&source, &binary, started.elapsed()) {
                    Ok(()) => Program::Native { path: binary },
                    Err(outcome) => return outcome,
                }
            }
            Language::Python => match runtime::find_interpreter(Language::Python) {
                Some(interpreter) => Program::Script {
                    program: interpreter.program,
                    run_args: interpreter.run_args,
                    script: source,
                },
                None => {
                    return RunOutcome::failure(
                        RunStatus::LaunchError,
                        "no Python interpreter was found on PATH",
                        started.elapsed().as_millis() as u64,
                    )
                }
            },
            Language::JavaScript => match runtime::find_interpreter(Language::JavaScript) {
                Some(interpreter) => Program::Script {
                    program: interpreter.program,
                    run_args: interpreter.run_args,
                    script: source,
                },
                None => {
                    return RunOutcome::failure(
                        RunStatus::LaunchError,
                        "no Node.js runtime was found on PATH",
                        started.elapsed().as_millis() as u64,
                    )
                }
            },
        };

        self.execute(program, &request.stdin, workspace.path(), started)
    }

    /// Compiles a Rust source file into a native binary.
    fn compile_rust(
        &self,
        source: &Path,
        binary: &Path,
        elapsed: Duration,
    ) -> Result<(), RunOutcome> {
        let rustc = runtime::find_rustc().ok_or_else(|| {
            RunOutcome::failure(
                RunStatus::LaunchError,
                "no Rust compiler (`rustc`) was found on PATH",
                elapsed.as_millis() as u64,
            )
        })?;

        let mut cmd = Command::new(&rustc.program);
        cmd.arg("--edition")
            .arg("2021")
            .arg("-C")
            .arg("opt-level=0")
            .arg("-C")
            .arg("debuginfo=0")
            .arg("-o")
            .arg(binary)
            .arg(source);
        if let Some(parent) = source.parent() {
            cmd.current_dir(parent);
        }

        let limit = self.limits.output_limit_bytes.max(256 * 1024);
        let compiled = capture_with_timeout(&mut cmd, COMPILE_TIMEOUT, limit);

        match compiled {
            Ok(result) if result.timed_out => Err(RunOutcome::failure(
                RunStatus::CompileError,
                "the Rust compiler timed out",
                result.duration_ms,
            )),
            Ok(result) if result.status.map(|status| status.success()) == Some(true) => Ok(()),
            Ok(result) => {
                let mut stderr = result.stderr;
                if stderr.trim().is_empty() {
                    stderr = result.stdout;
                }
                Err(RunOutcome {
                    stdout: String::new(),
                    stderr,
                    exit_code: result.status.and_then(|status| status.code()),
                    duration_ms: result.duration_ms,
                    status: RunStatus::CompileError,
                    truncated: result.truncated,
                })
            }
            Err(err) => Err(RunOutcome::failure(
                RunStatus::LaunchError,
                format!("could not start the Rust compiler: {err}"),
                elapsed.as_millis() as u64,
            )),
        }
    }

    /// Spawns the program and captures its output.
    fn execute(
        &self,
        program: Program,
        stdin_data: &str,
        cwd: &Path,
        started: Instant,
    ) -> RunOutcome {
        let is_node = matches!(&program, Program::Script { program, .. } if program == "node" || program == "nodejs");

        let mut cmd = match program {
            Program::Native { path } => Command::new(path),
            Program::Script {
                program,
                run_args,
                script,
            } => {
                let mut cmd = Command::new(program);
                cmd.args(&run_args);
                cmd.arg(script);
                cmd
            }
        };

        cmd.current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut limits = self.limits.clone();
        if is_node {
            // V8 pointer compression in 64-bit Node.js requires reserving at least 2 GiB
            // of virtual address space on initialization.
            limits.memory_bytes = limits.memory_bytes.max(2 * 1024 * 1024 * 1024);
            // Node.js spins up libuv threadpool (default 4 threads), V8 worker threads,
            // and delayed task schedulers on startup. Provide ample process/thread headroom.
            limits.max_processes = limits.max_processes.max(256);
        }

        let sandbox = match Sandbox::new(&limits) {
            Ok(sandbox) => Arc::new(sandbox),
            Err(err) => {
                return RunOutcome::failure(
                    RunStatus::LaunchError,
                    format!("could not configure the resource limits: {err}"),
                    started.elapsed().as_millis() as u64,
                )
            }
        };
        if let Err(err) = sandbox.apply(&mut cmd) {
            return RunOutcome::failure(
                RunStatus::LaunchError,
                format!("could not apply the resource limits: {err}"),
                started.elapsed().as_millis() as u64,
            );
        }

        let exec_started = Instant::now();
        let mut child: Child = match cmd.spawn() {
            Ok(child) => child,
            Err(err) => {
                return RunOutcome::failure(
                    RunStatus::SpawnError,
                    format!("could not start the program: {err}"),
                    started.elapsed().as_millis() as u64,
                )
            }
        };

        let pid = child.id();
        if let Err(err) = sandbox.attach(&child) {
            // Not fatal: the wall-clock watchdog still applies. Surfaced on the host's stderr.
            eprintln!("[sandbox] could not attach the process to the sandbox: {err}");
        }

        let output_limit = self.limits.output_limit_bytes;

        // Feed stdin from a dedicated thread: a program that never reads its input would
        // otherwise block the writer once the pipe buffer is full.
        let stdin_thread = child.stdin.take().map(|mut pipe| {
            let data = stdin_data.to_owned();
            std::thread::spawn(move || {
                use std::io::Write;
                let _ = pipe.write_all(data.as_bytes());
                let _ = pipe.flush();
            })
        });

        // Read both streams concurrently so a chatty program can never fill a pipe and deadlock.
        let stdout_thread = child.stdout.take().map(|pipe| {
            std::thread::spawn(move || read_capped(pipe, output_limit))
        });
        let stderr_thread = child.stderr.take().map(|pipe| {
            std::thread::spawn(move || read_capped(pipe, output_limit))
        });

        // Watchdog: kills the whole process tree when the wall-clock budget is exhausted.
        let timed_out = Arc::new(AtomicBool::new(false));
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let watchdog = {
            let timed_out = Arc::clone(&timed_out);
            let sandbox = Arc::clone(&sandbox);
            let timeout = self.limits.wall_timeout;
            std::thread::spawn(move || {
                if done_rx.recv_timeout(timeout).is_err() {
                    timed_out.store(true, Ordering::SeqCst);
                    sandbox.terminate(pid);
                }
            })
        };

        let status = child.wait();
        let _ = done_tx.send(());
        let _ = watchdog.join();

        let (stdout, stdout_truncated) = stdout_thread
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default();
        let (stderr, stderr_truncated) = stderr_thread
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default();
        if let Some(handle) = stdin_thread {
            let _ = handle.join();
        }

        let duration_ms = exec_started.elapsed().as_millis() as u64;

        match status {
            Ok(status) => {
                let was_timed_out = timed_out.load(Ordering::SeqCst);
                RunOutcome {
                    stdout,
                    stderr,
                    exit_code: status.code(),
                    duration_ms,
                    status: if was_timed_out {
                        RunStatus::TimedOut
                    } else {
                        RunStatus::Completed
                    },
                    truncated: stdout_truncated || stderr_truncated,
                }
            }
            Err(err) => RunOutcome::failure(
                RunStatus::SpawnError,
                format!("could not wait for the program: {err}"),
                duration_ms,
            ),
        }
    }
}

/// Reads at most `limit` bytes from `reader`, draining the rest so the child never blocks.
///
/// Returns the captured text and whether anything was dropped.
fn read_capped<R: Read>(mut reader: R, limit: usize) -> (String, bool) {
    let mut captured: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut truncated = false;

    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                if captured.len() < limit {
                    let take = (limit - captured.len()).min(count);
                    captured.extend_from_slice(&chunk[..take]);
                    if take < count {
                        truncated = true;
                    }
                } else {
                    truncated = true;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    (String::from_utf8_lossy(&captured).to_string(), truncated)
}

/// The result of a captured command.
struct CapturedCommand {
    stdout: String,
    stderr: String,
    status: Option<std::process::ExitStatus>,
    duration_ms: u64,
    truncated: bool,
    timed_out: bool,
}

/// Runs a command and captures its output, killing it after `timeout`.
fn capture_with_timeout(
    cmd: &mut Command,
    timeout: Duration,
    limit: usize,
) -> std::io::Result<CapturedCommand> {
    let started = Instant::now();
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_thread = stdout.map(|pipe| std::thread::spawn(move || read_capped(pipe, limit)));
    let stderr_thread = stderr.map(|pipe| std::thread::spawn(move || read_capped(pipe, limit)));

    let deadline = Instant::now() + timeout;
    let status;
    let mut timed_out = false;
    loop {
        match child.try_wait()? {
            Some(exit_status) => {
                status = Some(exit_status);
                break;
            }
            None => {
                if Instant::now() >= deadline {
                    timed_out = true;
                    let _ = child.kill();
                    status = child.wait().ok();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    let (stdout_text, stdout_truncated) = stdout_thread
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    let (stderr_text, stderr_truncated) = stderr_thread
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();

    Ok(CapturedCommand {
        stdout: stdout_text,
        stderr: stderr_text,
        status,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated: stdout_truncated || stderr_truncated,
        timed_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_capped_keeps_short_output_intact() {
        let (text, truncated) = read_capped("hello".as_bytes(), 1024);
        assert_eq!(text, "hello");
        assert!(!truncated);
    }

    #[test]
    fn read_capped_truncates_long_output() {
        let (text, truncated) = read_capped("x".repeat(5000).as_bytes(), 100);
        assert_eq!(text.len(), 100);
        assert!(truncated);
    }

    #[test]
    fn read_capped_handles_exact_limit() {
        let (text, truncated) = read_capped("y".repeat(100).as_bytes(), 100);
        assert_eq!(text.len(), 100);
        assert!(!truncated);
    }

    #[test]
    fn request_builder_sets_code_and_stdin() {
        let request = RunRequest::new(Language::Python, "print(1)").with_stdin("hello");
        assert_eq!(request.code, "print(1)");
        assert_eq!(request.stdin, "hello");
        assert_eq!(request.language, Language::Python);
    }

    #[test]
    fn runner_exposes_its_configuration() {
        let runner = Runner::new(std::env::temp_dir()).with_limits(Limits::permissive());
        assert_eq!(runner.limits(), &Limits::permissive());
        assert_eq!(runner.base_dir(), std::env::temp_dir().as_path());
    }
}
