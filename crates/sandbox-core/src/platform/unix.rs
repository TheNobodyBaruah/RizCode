//! Unix resource limits (`setrlimit`).
//!
//! The child is placed in its own process group (`setpgid(0, 0)`) which lets the watchdog kill
//! the whole process tree, and the rlimits are installed in a `pre_exec` hook — i.e. after `fork`
//! but before `exec`, which is the only point where the limits can still influence the new image.

use std::io;
use std::process::{Child, Command};

use crate::limits::{Limits, MAX_FILE_SIZE_BYTES};

/// A configured set of Unix limits.
#[derive(Debug, Clone)]
pub struct Sandbox {
    limits: Limits,
}

impl Sandbox {
    /// Prepares the sandbox. Always succeeds on Unix.
    pub fn new(limits: &Limits) -> io::Result<Self> {
        Ok(Self {
            limits: limits.clone(),
        })
    }

    /// Installs the process group and the rlimits on `cmd`.
    pub fn apply(&self, cmd: &mut Command) -> io::Result<()> {
        use std::os::unix::process::CommandExt;

        // A fresh process group makes the child its own group leader: killing `-pgid` then takes
        // down the program *and* every child it spawned.
        cmd.process_group(0);

        let limits = self.limits.clone();
        let max_file_size = MAX_FILE_SIZE_BYTES;
        let headroom = limits.max_processes.max(512);
        let effective_nproc = current_user_task_count().saturating_add(headroom);

        // Safety: `pre_exec` runs in the forked child between `fork` and `exec`. Only
        // async-signal-safe operations are allowed here; `setrlimit` qualifies.
        unsafe {
            cmd.pre_exec(move || {
                set_limit(libc::RLIMIT_AS, limits.memory_bytes)?;
                set_limit(libc::RLIMIT_CPU, limits.cpu_seconds)?;
                set_limit(libc::RLIMIT_NPROC, effective_nproc)?;
                set_limit(libc::RLIMIT_FSIZE, max_file_size)?;
                Ok(())
            });
        }

        Ok(())
    }

    /// Nothing to do on Unix — the limits were installed before `exec`.
    pub fn attach(&self, _child: &Child) -> io::Result<()> {
        Ok(())
    }

    /// Kills the whole process group of `pid`.
    pub fn terminate(&self, pid: u32) {
        let pid = pid as libc::pid_t;
        if pid <= 0 {
            return;
        }
        // Safety: `kill` is async-signal-safe and only fails if the group is already gone.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
            libc::kill(pid, libc::SIGKILL);
        }
    }
}

/// Installs a single rlimit.
fn set_limit(resource: libc::__rlimit_resource_t, value: u64) -> io::Result<()> {
    let limit = libc::rlimit {
        rlim_cur: value as libc::rlim_t,
        rlim_max: value as libc::rlim_t,
    };
    // Safety: `limit` is a valid, fully initialised `rlimit` structure.
    let result = unsafe { libc::setrlimit(resource, &limit) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Returns the current number of tasks (threads/processes) owned by the current UID.
/// On Linux, `RLIMIT_NPROC` is enforced per-UID across the entire machine; setting it to a value
/// lower than or equal to existing UID tasks prevents the child from creating *any* threads
/// (such as Node.js worker pools) or child processes. Adding the existing count ensures the child
/// receives an allowance of `limits.max_processes` additional processes.
fn current_user_task_count() -> u64 {
    #[cfg(target_os = "linux")]
    {
        let my_uid = unsafe { libc::getuid() };
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.chars().all(|c| c.is_ascii_digit()) {
                        use std::os::unix::fs::MetadataExt;
                        if let Ok(meta) = entry.metadata() {
                            if meta.uid() == my_uid {
                                if let Ok(tasks) = std::fs::read_dir(entry.path().join("task")) {
                                    count += tasks.count() as u64;
                                } else {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        if count > 0 {
            return count;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;
    use std::process::Stdio;

    #[test]
    fn applies_rlimits_to_a_child_process() {
        let limits = Limits {
            memory_bytes: 256 * 1024 * 1024,
            cpu_seconds: 7,
            max_processes: 32,
            ..Limits::default()
        };
        let sandbox = Sandbox::new(&limits).unwrap();

        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c")
            .arg("ulimit -v; ulimit -t")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        sandbox.apply(&mut cmd).unwrap();

        let output = cmd.output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines();
        // `ulimit -v` is reported in KiB.
        let address_space = lines.next().unwrap_or_default().trim().parse::<u64>().unwrap();
        assert_eq!(address_space, 256 * 1024);
        let cpu = lines.next().unwrap_or_default().trim().parse::<u64>().unwrap();
        assert_eq!(cpu, 7);
    }

    #[test]
    fn child_runs_in_its_own_process_group() {
        let sandbox = Sandbox::new(&Limits::default()).unwrap();
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c")
            .arg("ps -o pid=,pgid= -p $$")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        sandbox.apply(&mut cmd).unwrap();
        let output = cmd.output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8_lossy(&output.stdout);
        let mut parts = text.split_whitespace();
        let pid = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let pgid = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        assert!(pid > 0);
        assert_eq!(pid, pgid, "child process must be its own process group leader");
    }

    #[test]
    fn terminate_kills_a_running_process_group() {
        let sandbox = Sandbox::new(&Limits::default()).unwrap();
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c")
            .arg("sleep 30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        sandbox.apply(&mut cmd).unwrap();
        let mut child = cmd.spawn().unwrap();
        let pid = child.id();
        std::thread::sleep(std::time::Duration::from_millis(150));
        sandbox.terminate(pid);
        let status = child.wait().unwrap();
        assert!(!status.success());
    }
}
