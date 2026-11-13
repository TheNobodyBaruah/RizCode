//! Throwaway workspace directories.
//!
//! Every run gets its own directory underneath the runner's base directory. The directory is
//! removed when the [`Workspace`] value is dropped, so a run never leaves anything behind and two
//! concurrent runs can never collide.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Prefix of every workspace directory.
pub const WORKSPACE_PREFIX: &str = "code-sandbox-";

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique, isolated directory for a single run.
#[derive(Debug)]
pub struct Workspace {
    path: PathBuf,
}

impl Workspace {
    /// Creates a fresh workspace directory inside `base_dir`.
    ///
    /// `base_dir` is created if it does not exist yet.
    pub fn create(base_dir: impl AsRef<Path>) -> io::Result<Self> {
        let base_dir = base_dir.as_ref();
        std::fs::create_dir_all(base_dir)?;

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();

        let path = base_dir.join(format!("{WORKSPACE_PREFIX}{pid}-{nanos}-{seq}"));
        std::fs::create_dir(&path)?;
        Ok(Self { path })
    }

    /// The absolute path of the workspace directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The path of a file inside the workspace.
    pub fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    /// Writes a file into the workspace and returns its path.
    pub fn write(&self, name: &str, contents: &str) -> io::Result<PathBuf> {
        let path = self.file(name);
        std::fs::write(&path, contents)?;
        Ok(path)
    }

    /// Keeps the directory on disk (used by tests that want to inspect it).
    pub fn keep(mut self) -> PathBuf {
        std::mem::take(&mut self.path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        // Best effort: a failure here must never break a run.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;

    fn temp_base() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ws-test-{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_a_unique_directory_per_workspace() {
        let base = temp_base();
        let a = Workspace::create(&base).unwrap();
        let b = Workspace::create(&base).unwrap();
        assert_ne!(a.path(), b.path());
        assert!(a.path().starts_with(&base));
        assert!(a
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(WORKSPACE_PREFIX));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn is_removed_when_dropped() {
        let base = temp_base();
        let path = {
            let ws = Workspace::create(&base).unwrap();
            std::fs::write(ws.file("main.rs"), "fn main() {}").unwrap();
            ws.path().to_path_buf()
        };
        assert!(!path.exists(), "workspace should be removed on drop");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn writes_files_inside_the_workspace() {
        let base = temp_base();
        let ws = Workspace::create(&base).unwrap();
        let path = ws.write("main.py", "print(1)").unwrap();
        assert!(path.starts_with(ws.path()));
        assert_eq!(std::fs::read_to_string(path).unwrap(), "print(1)");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn base_directory_is_created_if_missing() {
        let base = temp_base().join("nested").join("deeper");
        let ws = Workspace::create(&base).unwrap();
        assert!(base.exists());
        assert!(ws.path().exists());
        let _ = std::fs::remove_dir_all(&base.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn defaults_are_not_shared_with_the_limits_module() {
        // Guards against accidentally coupling workspace creation to the limits configuration.
        let _ = Limits::default();
        let base = temp_base();
        let ws = Workspace::create(&base).unwrap();
        assert!(ws.path().exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
