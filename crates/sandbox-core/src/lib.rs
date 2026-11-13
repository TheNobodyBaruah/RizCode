//! # sandbox-core
//!
//! The execution engine of the Code Sandbox desktop application.
//!
//! This crate deliberately has **no** GUI or webview dependency: it only knows about languages,
//! resource limits and processes. That makes the risky part of the application testable with a
//! plain `cargo test` on any machine — including CI containers without a display server.
//!
//! ```no_run
//! use sandbox_core::{Language, RunRequest, Runner};
//!
//! let runner = Runner::new(std::env::temp_dir());
//! let outcome = runner.run(&RunRequest::new(Language::Python, "print('hi')"));
//! assert_eq!(outcome.stdout.trim(), "hi");
//! ```

pub mod fs;
pub mod language;
pub mod limits;
pub mod platform;
pub mod runner;
pub mod runtime;
pub mod workspace;

pub use fs::{atomic_write, read_to_string};
pub use language::Language;
pub use limits::Limits;
pub use runner::{RunOutcome, RunRequest, RunStatus, Runner};
pub use runtime::{detect_runtimes, ProgramInfo, RuntimeInfo};
pub use workspace::Workspace;

/// The version of the sandbox core crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
