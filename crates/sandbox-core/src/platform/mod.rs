//! Platform dispatch for resource limits.
//!
//! The sandbox exposes the *same* behaviour on every platform, but the enforcement mechanism
//! differs:
//!
//! * **Unix** (Linux, macOS) — `setrlimit(2)` applied to the child process before `exec`, plus a
//!   new process group so the whole tree can be killed at once.
//! * **Windows** — a Job Object with `JOB_OBJECT_LIMIT_*` flags; the child is assigned to the job
//!   right after it is created and `TerminateJobObject` kills the whole tree.
//!
//! Both back-ends live behind the [`Sandbox`] type so the runner is platform-agnostic.

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub use unix::Sandbox;
#[cfg(windows)]
pub use windows::Sandbox;

/// A short description of the active limit back-end (used in diagnostics and tests).
pub const BACKEND: &str = if cfg!(unix) {
    "rlimits"
} else if cfg!(windows) {
    "job-objects"
} else {
    "none"
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backend_is_reported() {
        assert!(matches!(BACKEND, "rlimits" | "job-objects" | "none"));
        if cfg!(any(unix, windows)) {
            assert_ne!(BACKEND, "none");
        }
    }
}
