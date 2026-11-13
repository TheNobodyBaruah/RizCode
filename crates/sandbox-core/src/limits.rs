//! Resource limits applied to every executed program.

use std::time::Duration;

/// Default wall-clock timeout for a program (5 seconds).
pub const DEFAULT_WALL_TIMEOUT_MS: u64 = 5_000;
/// Default address-space limit (512 MiB).
pub const DEFAULT_MEMORY_BYTES: u64 = 512 * 1024 * 1024;
/// Default CPU-time limit (5 seconds).
pub const DEFAULT_CPU_SECONDS: u64 = 5;
/// Default cap on captured stdout/stderr (1 MiB, each).
pub const DEFAULT_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;
/// Default cap on the number of processes in the job (64).
pub const DEFAULT_MAX_PROCESSES: u64 = 64;
/// Maximum size of a file the sandboxed program may create (64 MiB).
///
/// Not part of [`Limits`] because it is a safety net rather than a user-facing knob; it is
/// enforced through `RLIMIT_FSIZE` / Job Object file limits.
pub const MAX_FILE_SIZE_BYTES: u64 = 64 * 1024 * 1024;
/// Timeout applied to the `rustc` compilation step.
pub const COMPILE_TIMEOUT: Duration = Duration::from_secs(30);

/// The resource limits applied to a sandboxed program.
///
/// The same struct is used on every platform; the enforcement mechanism differs (rlimits on
/// Unix, Job Objects on Windows) but the *observable* behaviour is identical.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Limits {
    /// Maximum wall-clock runtime of the program.
    pub wall_timeout: Duration,
    /// Maximum address space (Unix) / committed memory (Windows) of the program.
    pub memory_bytes: u64,
    /// Maximum CPU time consumed by the program.
    pub cpu_seconds: u64,
    /// Maximum number of bytes captured per output stream.
    pub output_limit_bytes: usize,
    /// Maximum number of processes that may exist in the sandbox at once.
    pub max_processes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            wall_timeout: Duration::from_millis(DEFAULT_WALL_TIMEOUT_MS),
            memory_bytes: DEFAULT_MEMORY_BYTES,
            cpu_seconds: DEFAULT_CPU_SECONDS,
            output_limit_bytes: DEFAULT_OUTPUT_LIMIT_BYTES,
            max_processes: DEFAULT_MAX_PROCESSES,
        }
    }
}

impl Limits {
    /// A permissive set of limits, useful for tests that must not be killed by the sandbox.
    pub fn permissive() -> Self {
        Self {
            wall_timeout: Duration::from_secs(60),
            memory_bytes: 2 * 1024 * 1024 * 1024,
            cpu_seconds: 60,
            output_limit_bytes: 8 * 1024 * 1024,
            max_processes: 256,
        }
    }

    /// The wall-clock timeout expressed in milliseconds.
    pub fn wall_timeout_ms(&self) -> u64 {
        self.wall_timeout.as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_documented_values() {
        let limits = Limits::default();
        assert_eq!(limits.wall_timeout, Duration::from_secs(5));
        assert_eq!(limits.memory_bytes, 512 * 1024 * 1024);
        assert_eq!(limits.cpu_seconds, 5);
        assert_eq!(limits.output_limit_bytes, 1024 * 1024);
        assert_eq!(limits.max_processes, 64);
    }

    #[test]
    fn wall_timeout_is_reported_in_milliseconds() {
        let limits = Limits {
            wall_timeout: Duration::from_millis(1500),
            ..Limits::default()
        };
        assert_eq!(limits.wall_timeout_ms(), 1500);
    }

    #[test]
    fn limits_round_trip_through_serde() {
        let limits = Limits::default();
        let json = serde_json::to_string(&limits).unwrap();
        let parsed: Limits = serde_json::from_str(&json).unwrap();
        assert_eq!(limits, parsed);
    }

    #[test]
    fn permissive_limits_are_larger_than_the_defaults() {
        let permissive = Limits::permissive();
        assert!(permissive.wall_timeout > Limits::default().wall_timeout);
        assert!(permissive.memory_bytes > Limits::default().memory_bytes);
        assert!(permissive.output_limit_bytes > Limits::default().output_limit_bytes);
    }
}
