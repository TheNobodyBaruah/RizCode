//! Small, dependency-free file helpers.
//!
//! The only interesting one is [`atomic_write`]: writing a file directly can leave a truncated
//! file behind if the process dies halfway through. Writing to a sibling temporary file and
//! renaming it over the target makes the operation all-or-nothing.

use std::io;
use std::path::Path;

/// Writes `contents` to `path` atomically.
///
/// The data is written to a temporary file in the *same directory* as `path` (so the rename
/// never crosses a file-system boundary), flushed, and then renamed over the destination.
/// On Windows the destination is removed first because `std::fs::rename` cannot replace an
/// existing file there.
pub fn atomic_write(path: &Path, contents: &str) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    let temp_path = parent.join(format!(".{file_name}.{}.tmp", unique_suffix()));
    let temp_file = std::fs::File::create(&temp_path)?;
    {
        use std::io::Write;
        let mut handle = temp_file;
        handle.write_all(contents.as_bytes())?;
        handle.flush()?;
        handle.sync_all()?;
    }

    // Windows: `rename` fails if the destination exists.
    #[cfg(windows)]
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }

    match std::fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = std::fs::remove_file(&temp_path);
            Err(err)
        }
    }
}

/// Reads a file as UTF-8 text.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path)
}

/// A monotonically increasing suffix that makes temporary file names unique.
fn unique_suffix() -> u128 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    nanos.wrapping_mul(1_000).wrapping_add(seq as u128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fs-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_content_exactly() {
        let dir = temp_dir();
        let target = dir.join("out.txt");
        atomic_write(&target, "hello wörld\n").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello wörld\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn creates_missing_parent_directories() {
        let dir = temp_dir();
        let target = dir.join("a").join("b").join("out.txt");
        atomic_write(&target, "x").unwrap();
        assert_eq!(read_to_string(&target).unwrap(), "x");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn overwrites_an_existing_file() {
        let dir = temp_dir();
        let target = dir.join("out.txt");
        atomic_write(&target, "first").unwrap();
        atomic_write(&target, "second").unwrap();
        assert_eq!(read_to_string(&target).unwrap(), "second");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn leaves_no_temporary_files_behind() {
        let dir = temp_dir();
        let target = dir.join("out.txt");
        atomic_write(&target, "data").unwrap();
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| name.contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "unexpected temp files: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preserves_line_endings_exactly() {
        let dir = temp_dir();
        let target = dir.join("out.txt");
        atomic_write(&target, "a\r\nb\nc").unwrap();
        assert_eq!(read_to_string(&target).unwrap(), "a\r\nb\nc");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trips_empty_content() {
        let dir = temp_dir();
        let target = dir.join("empty.txt");
        atomic_write(&target, "").unwrap();
        assert_eq!(read_to_string(&target).unwrap(), "");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
