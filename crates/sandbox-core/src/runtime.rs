//! Runtime discovery.
//!
//! Nothing in the sandbox hardcodes an interpreter path. Every toolchain is probed at runtime so
//! the same binary works on Windows (`py -3`, `python`, `node`), macOS and Linux
//! (`python3`, `python`, `node`) without any platform-specific configuration.

use std::ffi::OsString;
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

/// Probes the given candidates and returns the first one that answers a version query.
fn probe(candidates: &[Candidate]) -> Option<(ProgramInfo, String)> {
    for candidate in candidates {
        let probe_args: Vec<OsString> =
            candidate.probe_args.iter().map(OsString::from).collect();
        if let Some(version) = run_version(candidate.program, &probe_args) {
            return Some((
                ProgramInfo {
                    program: OsString::from(candidate.program),
                    run_args: candidate.run_args.iter().map(OsString::from).collect(),
                },
                version,
            ));
        }
    }
    None
}

/// Runs `<program> <args>` and returns its trimmed first line of output.
fn run_version(program: &str, args: &[OsString]) -> Option<String> {
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
}
