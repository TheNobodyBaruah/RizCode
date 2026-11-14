//! Runtime discovery.
//!
//! Nothing in the sandbox hardcodes an interpreter path. Every toolchain is probed at runtime so
//! the same binary works on Windows (`py -3`, `python`, `node`), macOS and Linux
//! (`python3`, `python`, `node`) without any platform-specific configuration.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::language::Language;

/// A candidate toolchain: the executable, the arguments used to ask for its version and the
/// arguments that must precede a script when it is executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    /// The executable to look for.
    pub program: &'static str,
    /// Arguments used to query the version.
    pub probe_args: &'static [&'static str],
    /// Arguments that are passed in front of the script when running it.
    pub run_args: &'static [&'static str],
}

/// A resolved program plus the arguments that must be passed before the script path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramInfo {
    /// The executable to spawn.
    pub program: OsString,
    /// Arguments that always precede the user script.
    pub run_args: Vec<OsString>,
}

/// Availability information for one language toolchain.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuntimeInfo {
    /// The language this entry describes.
    pub language: Language,
    /// Whether a usable toolchain was found.
    pub available: bool,
    /// The program that would be used (display name).
    pub program: Option<String>,
    /// The version string reported by the toolchain.
    pub version: Option<String>,
    /// A human readable explanation when the toolchain is missing.
    pub message: Option<String>,
}

impl RuntimeInfo {
    fn missing(language: Language, message: impl Into<String>) -> Self {
        Self {
            language,
            available: false,
            program: None,
            version: None,
            message: Some(message.into()),
        }
    }
}

/// Candidate interpreters for a language, in probe order.
pub fn candidates(language: Language) -> &'static [Candidate] {
    match language {
        Language::Rust => &[Candidate {
            program: "rustc",
            probe_args: &["--version"],
            run_args: &[],
        }],
        Language::Python => {
            #[cfg(windows)]
            {
                &[
                    Candidate {
                        program: "py",
                        probe_args: &["-3", "--version"],
                        run_args: &["-3", "-B"],
                    },
                    Candidate {
                        program: "python",
                        probe_args: &["--version"],
                        run_args: &["-B"],
                    },
                    Candidate {
                        program: "python3",
                        probe_args: &["--version"],
                        run_args: &["-B"],
                    },
                ]
            }
            #[cfg(not(windows))]
            {
                &[
                    Candidate {
                        program: "python3",
                        probe_args: &["--version"],
                        run_args: &["-B"],
                    },
                    Candidate {
                        program: "python",
                        probe_args: &["--version"],
                        run_args: &["-B"],
                    },
                ]
            }
        }
        Language::JavaScript => &[
            Candidate {
                program: "node",
                probe_args: &["--version"],
                run_args: &[],
            },
            Candidate {
                program: "nodejs",
                probe_args: &["--version"],
                run_args: &[],
            },
        ],
    }
}

/// Prepends `dir` to the current process's `PATH` environment variable if not already present.
fn ensure_in_path(dir: &Path) {
    if let Ok(current) = std::env::var("PATH") {
        let mut paths: Vec<PathBuf> = std::env::split_paths(&current).collect();
        if !paths.iter().any(|p| p == dir) {
            paths.insert(0, dir.to_path_buf());
            if let Ok(new_path) = std::env::join_paths(paths) {
                unsafe {
                    std::env::set_var("PATH", new_path);
                }
            }
        }
    }
}

/// Attempts to resolve an executable name against `PATH` to an absolute path.
fn resolve_executable_path(name: &str) -> Option<PathBuf> {
    let name_path = Path::new(name);
    if name_path.is_absolute() || name_path.components().count() > 1 {
        if name_path.is_file() {
            return Some(name_path.to_path_buf());
        }
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let full = dir.join(name);
        if full.is_file() {
            return Some(full);
        }
        #[cfg(windows)]
        {
            let full_exe = dir.join(format!("{name}.exe"));
            if full_exe.is_file() {
                return Some(full_exe);
            }
        }
    }
    None
}

/// Returns fallback locations on disk to search if `program` is not found on the active `PATH`.
fn find_fallback_paths(program: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    match program {
        "rustc" => {
            if let Some(cargo_home) = std::env::var_os("CARGO_HOME").map(PathBuf::from) {
                paths.push(cargo_home.join("bin").join(if cfg!(windows) { "rustc.exe" } else { "rustc" }));
            }
            if let Some(ref h) = home {
                paths.push(h.join(".cargo").join("bin").join(if cfg!(windows) { "rustc.exe" } else { "rustc" }));
                let rustup_toolchains = h.join(".rustup").join("toolchains");
                if let Ok(entries) = std::fs::read_dir(&rustup_toolchains) {
                    for entry in entries.flatten() {
                        paths.push(entry.path().join("bin").join(if cfg!(windows) { "rustc.exe" } else { "rustc" }));
                    }
                }
            }
            if let Some(rustup_home) = std::env::var_os("RUSTUP_HOME").map(PathBuf::from) {
                let toolchains = rustup_home.join("toolchains");
                if let Ok(entries) = std::fs::read_dir(&toolchains) {
                    for entry in entries.flatten() {
                        paths.push(entry.path().join("bin").join(if cfg!(windows) { "rustc.exe" } else { "rustc" }));
                    }
                }
            }
            #[cfg(unix)]
            {
                paths.push(PathBuf::from("/opt/homebrew/bin/rustc"));
                paths.push(PathBuf::from("/usr/local/bin/rustc"));
                paths.push(PathBuf::from("/usr/bin/rustc"));
                paths.push(PathBuf::from("/opt/local/bin/rustc"));
                paths.push(PathBuf::from("/usr/local/cargo/bin/rustc"));
            }
        }
        "node" | "nodejs" => {
            if let Some(nvm_bin) = std::env::var_os("NVM_BIN").map(PathBuf::from) {
                paths.push(nvm_bin.join(if cfg!(windows) { "node.exe" } else { "node" }));
            }
            if let Some(ref h) = home {
                let nvm_versions = h.join(".nvm").join("versions").join("node");
                if let Ok(entries) = std::fs::read_dir(&nvm_versions) {
                    let mut version_dirs: Vec<PathBuf> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.is_dir())
                        .collect();
                    version_dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
                    for vdir in version_dirs {
                        paths.push(vdir.join("bin").join(if cfg!(windows) { "node.exe" } else { "node" }));
                    }
                }
                paths.push(h.join(".local").join("bin").join(if cfg!(windows) { "node.exe" } else { "node" }));
                paths.push(h.join(".fnm").join("current").join("bin").join(if cfg!(windows) { "node.exe" } else { "node" }));
                paths.push(h.join(".volta").join("bin").join(if cfg!(windows) { "node.exe" } else { "node" }));
            }
            #[cfg(unix)]
            {
                paths.push(PathBuf::from("/opt/homebrew/bin/node"));
                paths.push(PathBuf::from("/usr/local/bin/node"));
                paths.push(PathBuf::from("/usr/bin/node"));
                paths.push(PathBuf::from("/opt/local/bin/node"));
            }
            #[cfg(windows)]
            {
                if let Some(pf) = std::env::var_os("ProgramFiles").map(PathBuf::from) {
                    paths.push(pf.join("nodejs").join("node.exe"));
                }
                if let Some(pfx86) = std::env::var_os("ProgramFiles(x86)").map(PathBuf::from) {
                    paths.push(pfx86.join("nodejs").join("node.exe"));
                }
                if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
                    paths.push(appdata.join("npm").join("node.exe"));
                }
            }
        }
        "python3" | "python" => {
            #[cfg(unix)]
            {
                paths.push(PathBuf::from(format!("/opt/homebrew/bin/{program}")));
                paths.push(PathBuf::from(format!("/usr/local/bin/{program}")));
                paths.push(PathBuf::from(format!("/usr/bin/{program}")));
                paths.push(PathBuf::from(format!("/opt/local/bin/{program}")));
                if let Some(ref h) = home {
                    paths.push(h.join(".local").join("bin").join(program));
                }
            }
            #[cfg(windows)]
            {
                if let Some(local_appdata) = std::env::var_os("LocalAppData").map(PathBuf::from) {
                    let py_dir = local_appdata.join("Programs").join("Python");
                    if let Ok(entries) = std::fs::read_dir(&py_dir) {
                        let mut dirs: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
                        dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
                        for d in dirs {
                            paths.push(d.join("python.exe"));
                        }
                    }
                }
                if let Some(pf) = std::env::var_os("ProgramFiles").map(PathBuf::from) {
                    if let Ok(entries) = std::fs::read_dir(&pf) {
                        for entry in entries.flatten() {
                            if entry.file_name().to_string_lossy().starts_with("Python") {
                                paths.push(entry.path().join("python.exe"));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    paths
}

/// Probes the given candidates and returns the first one that answers a version query.
pub fn probe_with_path(
    candidates: &[Candidate],
    path_override: Option<&OsStr>,
) -> Option<(ProgramInfo, String)> {
    for candidate in candidates {
        let probe_args: Vec<OsString> =
            candidate.probe_args.iter().map(OsString::from).collect();

        // 1. Try resolving or running with PATH
        let candidate_path = match path_override {
            Some(custom_path) => {
                let mut found = None;
                for dir in std::env::split_paths(custom_path) {
                    let full = dir.join(candidate.program);
                    if full.is_file() {
                        found = Some(full);
                        break;
                    }
                    #[cfg(windows)]
                    {
                        let full_exe = dir.join(format!("{}.exe", candidate.program));
                        if full_exe.is_file() {
                            found = Some(full_exe);
                            break;
                        }
                    }
                }
                found
            }
            None => resolve_executable_path(candidate.program),
        };

        if let Some(ref exe_path) = candidate_path {
            if let Some(version) = run_version(exe_path.as_os_str(), &probe_args) {
                if let Some(parent) = exe_path.parent() {
                    ensure_in_path(parent);
                }
                return Some((
                    ProgramInfo {
                        program: exe_path.clone().into_os_string(),
                        run_args: candidate.run_args.iter().map(OsString::from).collect(),
                    },
                    version,
                ));
            }
        } else if path_override.is_none() {
            // Also try spawning bare name in case OS searches standard paths
            if let Some(version) = run_version(OsStr::new(candidate.program), &probe_args) {
                return Some((
                    ProgramInfo {
                        program: OsString::from(candidate.program),
                        run_args: candidate.run_args.iter().map(OsString::from).collect(),
                    },
                    version,
                ));
            }
        }

        // 2. Fallback search when not found on PATH
        for fallback in find_fallback_paths(candidate.program) {
            if fallback.is_file() {
                if let Some(version) = run_version(fallback.as_os_str(), &probe_args) {
                    if let Some(parent) = fallback.parent() {
                        ensure_in_path(parent);
                    }
                    return Some((
                        ProgramInfo {
                            program: fallback.into_os_string(),
                            run_args: candidate.run_args.iter().map(OsString::from).collect(),
                        },
                        version,
                    ));
                }
            }
        }
    }
    None
}

/// Probes the given candidates and returns the first one that answers a version query.
fn probe(candidates: &[Candidate]) -> Option<(ProgramInfo, String)> {
    probe_with_path(candidates, None)
}

/// Runs `<program> <args>` and returns its trimmed first line of output.
fn run_version(program: &OsStr, args: &[OsString]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let first = text.lines().next().unwrap_or_default().trim().to_string();
    if first.is_empty() {
        None
    } else {
        Some(first)
    }
}

/// Finds the Rust compiler.
pub fn find_rustc() -> Option<ProgramInfo> {
    static RUSTC: OnceLock<Option<ProgramInfo>> = OnceLock::new();
    RUSTC
        .get_or_init(|| {
            probe(candidates(Language::Rust)).map(|(program, _)| program)
        })
        .clone()
}

/// Finds the interpreter used to run scripts of the given language.
pub fn find_interpreter(language: Language) -> Option<ProgramInfo> {
    match language {
        Language::Rust => find_rustc(),
        Language::Python => {
            static PYTHON: OnceLock<Option<ProgramInfo>> = OnceLock::new();
            PYTHON
                .get_or_init(|| {
                    probe(candidates(Language::Python)).map(|(program, _)| program)
                })
                .clone()
        }
        Language::JavaScript => {
            static NODE: OnceLock<Option<ProgramInfo>> = OnceLock::new();
            NODE.get_or_init(|| {
                probe(candidates(Language::JavaScript)).map(|(program, _)| program)
            })
            .clone()
        }
    }
}

/// Probes every supported language and reports its availability.
pub fn detect_runtimes() -> Vec<RuntimeInfo> {
    Language::ALL
        .iter()
        .map(|&language| {
            let candidates = candidates(language);
            match probe(candidates) {
                Some((program, version)) => RuntimeInfo {
                    language,
                    available: true,
                    program: Some(program.program.to_string_lossy().to_string()),
                    version: Some(version),
                    message: None,
                },
                None => {
                    let names: Vec<&str> = candidates.iter().map(|c| c.program).collect();
                    RuntimeInfo::missing(
                        language,
                        format!("`{}` was not found on PATH", names.join("`, `")),
                    )
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_one_entry_per_language() {
        let infos = detect_runtimes();
        assert_eq!(infos.len(), Language::ALL.len());
        assert_eq!(infos[0].language, Language::Rust);
        assert_eq!(infos[1].language, Language::Python);
        assert_eq!(infos[2].language, Language::JavaScript);
    }

    #[test]
    fn available_runtimes_report_a_version() {
        for info in detect_runtimes() {
            if info.available {
                assert!(info.version.is_some(), "{:?} missing version", info.language);
                assert!(info.program.is_some(), "{:?} missing program", info.language);
            } else {
                assert!(info.message.is_some(), "{:?} missing message", info.language);
            }
        }
    }

    #[test]
    fn python_candidates_include_both_spellings() {
        let names: Vec<&str> = candidates(Language::Python)
            .iter()
            .map(|candidate| candidate.program)
            .collect();
        assert!(names.contains(&"python3"));
        assert!(names.contains(&"python"));
    }

    #[test]
    fn python_is_run_with_the_no_bytecode_flag() {
        for candidate in candidates(Language::Python) {
            assert!(
                candidate.run_args.contains(&"-B"),
                "{} should run with -B",
                candidate.program
            );
        }
    }

    #[test]
    fn javascript_candidates_include_node() {
        let names: Vec<&str> = candidates(Language::JavaScript)
            .iter()
            .map(|candidate| candidate.program)
            .collect();
        assert!(names.contains(&"node"));
    }

    #[test]
    fn rust_candidates_include_rustc() {
        let names: Vec<&str> = candidates(Language::Rust)
            .iter()
            .map(|candidate| candidate.program)
            .collect();
        assert_eq!(names, vec!["rustc"]);
    }

    #[test]
    fn fallback_discovery_finds_tools_with_empty_or_minimal_path() {
        let minimal_path = std::ffi::OsStr::new("/nonexistent_directory_for_test");
        // Rustc fallback should discover rustc from cargo home or user home
        let rust_result = probe_with_path(candidates(Language::Rust), Some(minimal_path));
        assert!(rust_result.is_some(), "rustc fallback should resolve even with minimal PATH");
        let (info, version) = rust_result.unwrap();
        assert!(version.contains("rustc"));
        assert!(info.program.to_string_lossy().contains("rustc"));

        // Node fallback should discover node from nvm or local
        let js_result = probe_with_path(candidates(Language::JavaScript), Some(minimal_path));
        assert!(js_result.is_some(), "node fallback should resolve even with minimal PATH");
        let (js_info, js_version) = js_result.unwrap();
        assert!(js_version.contains('v'));
        assert!(js_info.program.to_string_lossy().contains("node"));
    }
}
