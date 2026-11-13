use std::fs;
use std::time::Duration;

use sandbox_core::{
    atomic_write, detect_runtimes, Language, Limits, RunRequest, RunStatus, Runner,
};

fn test_runner() -> Runner {
    Runner::new(std::env::temp_dir())
}

// -----------------------------------------------------------------------------
// 1. Basic execution across all three supported languages
// -----------------------------------------------------------------------------

#[test]
fn runs_rust_hello_world() {
    let runner = test_runner();
    let req = RunRequest::new(
        Language::Rust,
        r#"
fn main() {
    println!("hello from rustc");
}
"#,
    );
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed, "stderr: {}", outcome.stderr);
    assert_eq!(outcome.stdout.trim(), "hello from rustc");
    assert_eq!(outcome.exit_code, Some(0));
    assert!(!outcome.truncated);
}

#[test]
fn runs_python_hello_world() {
    let runner = test_runner();
    let req = RunRequest::new(Language::Python, "print('hello from python')");
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed, "stderr: {}", outcome.stderr);
    assert_eq!(outcome.stdout.trim(), "hello from python");
    assert_eq!(outcome.exit_code, Some(0));
    assert!(!outcome.truncated);
}

#[test]
fn runs_javascript_hello_world() {
    let runner = test_runner();
    let req = RunRequest::new(Language::JavaScript, "console.log('hello from node');");
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed, "stderr: {}", outcome.stderr);
    assert_eq!(outcome.stdout.trim(), "hello from node");
    assert_eq!(outcome.exit_code, Some(0));
    assert!(!outcome.truncated);
}

// -----------------------------------------------------------------------------
// 2. Standard input (stdin) echoing
// -----------------------------------------------------------------------------

#[test]
fn rust_echoes_stdin_verbatim() {
    let runner = test_runner();
    let code = r#"
use std::io::{self, Read};
fn main() {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).unwrap();
    print!("{buf}");
}
"#;
    let input = "hello from stdin!\nline 2 with symbols: !@#$%^&*()";
    let req = RunRequest::new(Language::Rust, code).with_stdin(input);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed, "stderr: {}", outcome.stderr);
    assert_eq!(outcome.stdout, input);
}

#[test]
fn python_echoes_stdin_verbatim() {
    let runner = test_runner();
    let code = "import sys\nsys.stdout.write(sys.stdin.read())";
    let input = "multiline\npython\nstdin\ntest\n";
    let req = RunRequest::new(Language::Python, code).with_stdin(input);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed, "stderr: {}", outcome.stderr);
    assert_eq!(outcome.stdout, input);
}

#[test]
fn javascript_echoes_stdin_verbatim() {
    let runner = test_runner();
    let code = r#"
const fs = require('fs');
const input = fs.readFileSync(0, 'utf-8');
process.stdout.write(input);
"#;
    let input = "javascript stdin payload 123456";
    let req = RunRequest::new(Language::JavaScript, code).with_stdin(input);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed, "stderr: {}", outcome.stderr);
    assert_eq!(outcome.stdout, input);
}

// -----------------------------------------------------------------------------
// 3. Separate stdout and stderr capture
// -----------------------------------------------------------------------------

#[test]
fn rust_captures_stdout_and_stderr_separately() {
    let runner = test_runner();
    let code = r#"
fn main() {
    print!("standard-out-msg\n");
    eprint!("standard-err-msg\n");
}
"#;
    let req = RunRequest::new(Language::Rust, code);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.stdout, "standard-out-msg\n");
    assert_eq!(outcome.stderr, "standard-err-msg\n");
}

#[test]
fn python_captures_stdout_and_stderr_separately() {
    let runner = test_runner();
    let code = r#"
import sys
sys.stdout.write("py-stdout\n")
sys.stderr.write("py-stderr\n")
"#;
    let req = RunRequest::new(Language::Python, code);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.stdout, "py-stdout\n");
    assert_eq!(outcome.stderr, "py-stderr\n");
}

#[test]
fn javascript_captures_stdout_and_stderr_separately() {
    let runner = test_runner();
    let code = r#"
process.stdout.write("js-stdout\n");
process.stderr.write("js-stderr\n");
"#;
    let req = RunRequest::new(Language::JavaScript, code);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.stdout, "js-stdout\n");
    assert_eq!(outcome.stderr, "js-stderr\n");
}

// -----------------------------------------------------------------------------
// 4. Exit codes propagation
// -----------------------------------------------------------------------------

#[test]
fn propagates_non_zero_exit_codes() {
    let runner = test_runner();

    // Python exit code 42
    let py_req = RunRequest::new(Language::Python, "import sys; sys.exit(42)");
    let outcome = runner.run(&py_req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.exit_code, Some(42));

    // Rust exit code 7
    let rs_req = RunRequest::new(Language::Rust, "fn main() { std::process::exit(7); }");
    let outcome = runner.run(&rs_req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.exit_code, Some(7));

    // JavaScript exit code 19
    let js_req = RunRequest::new(Language::JavaScript, "process.exit(19);");
    let outcome = runner.run(&js_req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.exit_code, Some(19));
}

// -----------------------------------------------------------------------------
// 5. Rust compilation errors
// -----------------------------------------------------------------------------

#[test]
fn rust_syntax_error_yields_compile_error() {
    let runner = test_runner();
    let bad_code = r#"
fn main() {
    this is completely invalid rust syntax $$$$$;
}
"#;
    let req = RunRequest::new(Language::Rust, bad_code);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::CompileError);
    assert!(outcome.stdout.is_empty());
    assert!(
        outcome.stderr.contains("error: expected"),
        "stderr should contain rustc error diagnostics, got: {}",
        outcome.stderr
    );
    assert_ne!(outcome.exit_code, Some(0));
}

// -----------------------------------------------------------------------------
// 6. Wall-clock timeout enforcement
// -----------------------------------------------------------------------------

#[test]
fn kills_infinite_loops_on_timeout() {
    let limits = Limits {
        wall_timeout: Duration::from_millis(600),
        ..Limits::default()
    };
    let runner = Runner::new(std::env::temp_dir()).with_limits(limits);

    // Python infinite loop
    let req = RunRequest::new(Language::Python, "while True:\n    pass");
    let start = std::time::Instant::now();
    let outcome = runner.run(&req);
    let elapsed = start.elapsed();

    assert_eq!(outcome.status, RunStatus::TimedOut);
    assert!(
        elapsed >= Duration::from_millis(500),
        "execution finished suspiciously fast: {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "execution took too long to terminate: {:?}",
        elapsed
    );
}

#[test]
fn rust_infinite_loop_times_out() {
    let limits = Limits {
        wall_timeout: Duration::from_millis(600),
        ..Limits::default()
    };
    let runner = Runner::new(std::env::temp_dir()).with_limits(limits);

    let req = RunRequest::new(
        Language::Rust,
        r#"
fn main() {
    loop {}
}
"#,
    );
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::TimedOut);
}

// -----------------------------------------------------------------------------
// 7. Output truncation
// -----------------------------------------------------------------------------

#[test]
fn output_above_cap_is_truncated_and_flagged() {
    let limits = Limits {
        output_limit_bytes: 512,
        ..Limits::default()
    };
    let runner = Runner::new(std::env::temp_dir()).with_limits(limits);

    let req = RunRequest::new(Language::Python, "print('A' * 4096)");
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert!(outcome.truncated, "outcome should be flagged as truncated");
    assert_eq!(outcome.stdout.len(), 512);
    assert!(outcome.stdout.chars().all(|c| c == 'A'));
}

// -----------------------------------------------------------------------------
// 8. Shell injection safety (programs spawned directly via argv, never shell)
// -----------------------------------------------------------------------------

#[test]
fn code_containing_shell_metacharacters_is_executed_literally() {
    let runner = test_runner();

    // Python printing shell injection sequences
    let code = r#"
evil_text = "$(whoami) `id` ; rm -rf / && echo pwned | cat"
print(evil_text)
"#;
    let req = RunRequest::new(Language::Python, code);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(
        outcome.stdout.trim(),
        "$(whoami) `id` ; rm -rf / && echo pwned | cat"
    );
}

#[test]
fn rust_containing_shell_metacharacters_is_literal() {
    let runner = test_runner();
    let code = r#"
fn main() {
    let s = "$(id) `cat /etc/passwd` && echo injection ; ";
    print!("{s}");
}
"#;
    let req = RunRequest::new(Language::Rust, code);
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed);
    assert_eq!(outcome.stdout, "$(id) `cat /etc/passwd` && echo injection ; ");
}

// -----------------------------------------------------------------------------
// 9. Workspace isolation and lifecycle
// -----------------------------------------------------------------------------

#[test]
fn workspaces_are_isolated_and_cleaned_up() {
    let base = std::env::temp_dir().join(format!("test-sandbox-cleanup-{}", std::process::id()));
    let _ = fs::create_dir_all(&base);
    let runner = Runner::new(&base);

    let req = RunRequest::new(Language::Python, "print('temp dir test')");
    let outcome = runner.run(&req);
    assert_eq!(outcome.status, RunStatus::Completed);

    // After the run finishes, the temporary workspace directory inside `base` should have been removed
    let entries: Vec<_> = fs::read_dir(&base)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        entries.is_empty(),
        "workspace directory should be cleaned up after run, found: {:?}",
        entries
    );
    let _ = fs::remove_dir_all(&base);
}

// -----------------------------------------------------------------------------
// 10. Atomic file writes and File I/O
// -----------------------------------------------------------------------------

#[test]
fn atomic_write_round_trips_and_preserves_content() {
    let dir = std::env::temp_dir().join(format!("test-atomic-write-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let file = dir.join("test_write.txt");

    let content = "first line\r\nsecond line\nthird line with UTF-8: 🚀 Crab 🦀";
    atomic_write(&file, content).expect("atomic_write should succeed");

    let read_back = fs::read_to_string(&file).expect("should read back");
    assert_eq!(read_back, content);

    // Overwrite with different content
    let updated = "updated completely\nno CRLF here";
    atomic_write(&file, updated).expect("atomic_write overwrite should succeed");
    assert_eq!(fs::read_to_string(&file).unwrap(), updated);

    // Create file in non-existent nested subdirectory
    let nested = dir.join("nested").join("deep").join("file.py");
    atomic_write(&nested, "print('nested')").expect("atomic_write should create parent dirs");
    assert_eq!(fs::read_to_string(&nested).unwrap(), "print('nested')");

    let _ = fs::remove_dir_all(&dir);
}

// -----------------------------------------------------------------------------
// 11. Runtime detection
// -----------------------------------------------------------------------------

#[test]
fn runtime_detection_finds_available_toolchains() {
    let runtimes = detect_runtimes();
    assert_eq!(runtimes.len(), 3);

    for r in &runtimes {
        match r.language {
            Language::Rust => {
                assert!(r.available, "rustc should be available in this test environment");
                assert!(r.version.as_ref().unwrap().contains("rustc"));
            }
            Language::Python => {
                assert!(r.available, "python should be available in this test environment");
                assert!(r.version.as_ref().unwrap().contains("Python"));
            }
            Language::JavaScript => {
                assert!(r.available, "node should be available in this test environment");
                assert!(r.version.as_ref().unwrap().contains('v'));
            }
        }
    }
}
