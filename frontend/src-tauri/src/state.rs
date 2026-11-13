//! Managed application state.

use std::path::PathBuf;

use sandbox_core::{Limits, Runner};

/// Everything the Tauri commands need to do their job.
pub struct AppState {
    /// The execution engine.
    pub runner: Runner,
    /// The directory the throwaway workspaces are created in.
    pub workspace_root: PathBuf,
}

impl AppState {
    /// Builds the state using the platform's temp directory.
    pub fn new() -> Self {
        let workspace_root = std::env::temp_dir().join("code-sandbox");
        Self {
            runner: Runner::new(&workspace_root).with_limits(Limits::default()),
            workspace_root,
        }
    }

    /// Builds the state with a custom workspace root and limits (used by the tests).
    pub fn with_workspace_root(root: impl Into<PathBuf>, limits: Limits) -> Self {
        let workspace_root = root.into();
        Self {
            runner: Runner::new(&workspace_root).with_limits(limits),
            workspace_root,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_uses_a_temp_workspace_root() {
        let state = AppState::new();
        assert!(state.workspace_root.starts_with(std::env::temp_dir()));
    }

    #[test]
    fn state_can_be_created_with_custom_limits() {
        let root = std::env::temp_dir().join("code-sandbox-state-test");
        let state = AppState::with_workspace_root(&root, Limits::permissive());
        assert_eq!(state.runner.limits(), &Limits::permissive());
        assert_eq!(state.workspace_root, root);
    }
}
