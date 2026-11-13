# Code Sandbox — Implementation Plan

A standalone, platform-agnostic desktop coding sandbox for **Rust**, **Python** and **JavaScript**,
built with **Tauri v2** (Rust backend) and a **React + TypeScript + CodeMirror 6** frontend.

---

## 1. Product goals

| # | Goal | Acceptance |
|---|------|-----------|
| G1 | IDE on the left, Input/Output on the right | Editor pane occupies the main left column; stdin + stdout/stderr panels are stacked in the right column |
| G2 | Syntax highlighting for Rust, Python, JavaScript | CodeMirror language modes switch with the selected language |
| G3 | VIM mode with full navigation | `@replit/codemirror-vim`, toggleable, `:w` **runs** the program (never saves) |
| G4 | Run code "on the fly" without saving to disk | Source lives in memory; every run executes from a throwaway temp file |
| G5 | Optional permanent save to a user-chosen path | Dedicated toolbar **Save** button → native OS save dialog (folder + filename) |
| G6 | Standalone desktop app, no server/browser needed | Self-contained binary per OS; Windows, macOS, Linux |
| G7 | Test-driven development | Every module has tests written before implementation |
| G8 | Decision transparency | Every decision recorded in `work.md` |
| G9 | Native clipboard copy/paste | `Ctrl+C` / `Ctrl+V` on Windows & Linux, `Cmd+C` / `Cmd+V` on macOS, inside the editor |

---

## 2. Decisions (summary — full log with rationale in [`work.md`](./work.md))

| # | Decision |
|---|---------|
| D1 | In VIM mode, `:w` **runs** the program; it never writes a file |
| D2 | Saving happens **only** through the dedicated toolbar **Save** button |
| D3 | The Save button opens the **native OS save dialog** (choose folder *and* type filename) |
| D4 | **Full platform parity** for resource limits: rlimits on Unix, Windows Job Objects on Windows |
| D5 | **Standalone, platform-agnostic** binary for Windows / macOS / Linux |
| D6 | All logic lives in a Tauri-free `sandbox-core` crate; the Tauri crate is a thin adapter |
| D7 | Programs are spawned directly with argv — **never** through a shell |
| D8 | CodeMirror 6 + React 19 + TypeScript + Vite (not Monaco/Ace) |
| D9 | VIM mode is **off** by default; toggled via toolbar button or `Ctrl+Shift+V` |
| D10 | Auto-run is **on** by default: 800 ms debounce after the last keystroke; `Ctrl+Enter` runs immediately |
| D11 | Rust is compiled with `rustc --edition 2021` (single file, no crates.io) |
| D12 | Interpreters are **discovered at runtime**, never hardcoded (`py -3` on Windows, `python3` on Unix, …) |
| D13 | Source files are written atomically (temp file + rename) to avoid partial saves |
| D14 | The app is *sandbox-like* (limits + temp dir), **not** a security boundary |
| D18 | Clipboard uses the platform-native modifier: `Mod` = `Cmd` on macOS, `Ctrl` on Windows/Linux |

---

## 3. Repository layout

```
code_sandbox/
├── plan.md                      # this file
├── work.md                      # decision log (required deliverable)
├── Cargo.toml                   # workspace root
├── .github/workflows/ci.yml     # windows / macos / linux matrix
├── crates/
│   ├── sandbox-core/            # pure Rust, zero Tauri deps → fully unit-testable
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs           # public API re-exports
│   │   │   ├── language.rs      # Language enum, detection, file naming
│   │   │   ├── limits.rs        # Limits config + defaults
│   │   │   ├── runner.rs        # compile + execute pipeline
│   │   │   ├── workspace.rs     # throwaway temp dir lifecycle
│   │   │   ├── fs.rs            # atomic write / read helpers
│   │   │   └── platform/
│   │   │       ├── mod.rs       # cfg dispatch
│   │   │       ├── unix.rs      # setrlimit limits
│   │   │       └── windows.rs   # Job Object limits
│   │   └── tests/               # integration tests (real rustc / python3 / node)
│   └── app/                     # Tauri v2 crate (thin adapter over sandbox-core)
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── capabilities/default.json
│       ├── icons/
│       └── src/{main.rs,lib.rs,commands.rs,state.rs}
└── frontend/
    ├── package.json
    ├── vite.config.ts
    ├── tsconfig.json
    ├── index.html
    └── src/
        ├── main.tsx
        ├── App.tsx
        ├── components/{Editor.tsx,Toolbar.tsx,InputPanel.tsx,OutputPanel.tsx,StatusBar.tsx}
        ├── lib/{api.ts,theme.ts,clipboard.ts,keymap.ts}
        ├── store/useSandbox.ts
        └── __tests__/
```

---

## 4. Architecture

### 4.1 Backend split (D6)

```
frontend (React)  ──invoke──▶  Tauri commands (crates/app)  ──▶  sandbox-core
                                                                  ├─ language.rs
                                                                  ├─ runner.rs
                                                                  ├─ limits.rs
                                                                  ├─ workspace.rs
                                                                  └─ platform/{unix,windows}.rs
```

*Why:* the execution engine — the riskiest part — has **no GUI or webkit dependency**, so it can be
exhaustively unit- and integration-tested with plain `cargo test` on any machine, including CI
containers without a display server. The Tauri layer stays a thin, near-trivial adapter.

### 4.2 Execution model

| Language | Compile step | Run step | Executable |
|---|---|---|---|
| Rust | `rustc --edition 2021 -o prog main.rs` | `./prog` | native binary in temp dir |
| Python | — | `python3 -B main.py` | interpreter |
| JavaScript | — | `node main.js` | interpreter |

* Rust compile failures are surfaced as `RunStatus::CompileError` with the compiler's stderr.
* `python3 -B` prevents `.pyc` litter; the traceback still names `main.py`.
* No shell is ever involved (D7): `Command::new(prog).args([...])`, so code content can never be
  interpreted as shell syntax.

### 4.3 Isolation and limits (D4)

| Limit | Linux/macOS | Windows |
|---|---|---|
| Wall-clock timeout | watchdog thread → `kill()` | watchdog thread → `TerminateJobObject` |
| Memory | `RLIMIT_AS` | Job Object `JOB_OBJECT_LIMIT_PROCESS_MEMORY` |
| CPU time | `RLIMIT_CPU` | Job Object `JOB_OBJECT_LIMIT_PROCESS_TIME` |
| File size | `RLIMIT_FSIZE` | Job Object `JOB_OBJECT_LIMIT_JOB_MEMORY`/file I/O limits |
| Process count | `RLIMIT_NPROC` | Job Object `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` |
| Kill on exit | process group kill | `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` |

Defaults: 5 s wall clock, 512 MiB address space, 5 s CPU, 1 MiB captured output, 64 processes.

Each run happens in a **fresh temp directory** (`std::env::temp_dir()/code-sandbox-<uuid>`) that is
removed afterwards, so runs never collide and never see the user's files.

### 4.4 Public API of `sandbox-core`

```rust
pub enum Language { Rust, Python, JavaScript }
impl Language {
    pub fn from_extension(ext: &str) -> Option<Self>;
    pub fn from_path(path: &Path) -> Option<Self>;
    pub fn file_name(&self) -> &'static str;      // main.rs / main.py / main.js
    pub fn extension(&self) -> &'static str;      // rs / py / js
    pub fn label(&self) -> &'static str;
}

pub struct RunRequest { pub language: Language, pub code: String, pub stdin: String }
pub struct RunOutcome {
    pub stdout: String, pub stderr: String,
    pub exit_code: Option<i32>, pub duration_ms: u64,
    pub status: RunStatus, pub truncated: bool,
}
pub enum RunStatus { Completed, CompileError, TimedOut, LaunchError, SpawnError }
pub struct Limits { pub wall_timeout: Duration, pub memory_bytes: u64,
                    pub cpu_seconds: u64, pub output_limit_bytes: usize,
                    pub max_processes: u64 }

pub struct Runner { base_dir: PathBuf, limits: Limits }
impl Runner {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self;
    pub fn with_limits(self, limits: Limits) -> Self;
    pub fn run(&self, req: &RunRequest) -> RunOutcome;
}

pub fn detect_runtimes() -> Vec<RuntimeInfo>;   // availability + version per language
pub fn atomic_write(path: &Path, contents: &str) -> io::Result<()>;
```

### 4.5 Tauri commands (thin)

| Command | Payload | Result |
|---|---|---|
| `run_code` | `{ language, code, stdin }` | `RunOutcome` (serde) |
| `save_file` | `{ path, contents }` | `{ path, bytes_written }` |
| `open_file` | `{ path }` | `{ path, contents }` |
| `detect_runtimes` | — | `Vec<RuntimeInfo>` |
| `app_info` | — | `{ version, platform }` |
| `clipboard_write_text` | `{ text }` | `()` |
| `clipboard_read_text` | — | `String` |

Save/Open dialogs are opened in the **frontend** via `tauri-plugin-dialog` (native panels), the
file IO itself goes through the backend so it is testable.

---

## 5. Frontend

* **Layout**: `flex` row → left column (toolbar + editor), right column (input panel, output panel).
  Panels are resizable by a drag handle (simple CSS/pointer-events implementation).
* **Editor**: CodeMirror 6 with `@codemirror/lang-rust`, `@codemirror/lang-python`,
  `@codemirror/lang-javascript`, a dark theme, line numbers, bracket matching, and
  `@replit/codemirror-vim` (toggleable).
* **VIM**: `:w` is bound to **run** (D1). A `Ctrl+Shift+V` shortcut toggles vim mode.
  The status bar shows the current mode (`VIM` / `NORMAL`) and a hint that `:w` runs.
* **Auto-run** (D10): 800 ms debounce on buffer change; a run in flight is superseded by the next
  one (sequence numbers). Toolbar toggle switches it off. `Ctrl+Enter` forces an immediate run.
* **Save** (D2/D3): toolbar button → `save()` from `@tauri-apps/plugin-dialog` with
  `defaultPath` = last used directory + `main.<ext>`; the returned path is sent to `save_file`.
  The header shows a dot when there are unsaved changes and the last saved path when clean.
* **Output**: stdout and stderr in separate collapsible areas, exit code + duration in the status bar.

### 5.1 Clipboard support (G9 / D18)

Copying and pasting must work with the **shortcuts the user already expects on their OS**:

| OS | Copy | Paste | Cut |
|---|---|---|---|
| Windows | `Ctrl+C` | `Ctrl+V` | `Ctrl+X` |
| Linux | `Ctrl+C` | `Ctrl+V` | `Ctrl+X` |
| macOS | `Cmd+C` | `Cmd+V` | `Cmd+X` |

**Design.** This is expressed as CodeMirror's `Mod` modifier — `Mod` resolves to `Cmd` on macOS and
to `Ctrl` everywhere else — so *one* binding covers all three platforms without OS sniffing:

* The editor installs an explicit keymap with `Mod-c` → copy, `Mod-v` → paste, `Mod-x` → cut,
  `Mod-a` → select all. These are declared in the frontend's own extension (not merely inherited
  from `basicSetup`) so the behaviour is **guaranteed, observable and unit-testable**.
* The bindings work for a selection *and* for the empty selection (no selection + `Ctrl+C` copies
  the whole current line, matching every other editor).
* When VIM mode is active, the VIM keymap takes precedence in `NORMAL`/`VISUAL` mode (so `y`/`p`
  work as users expect), but `Mod-c` / `Mod-v` remain bound and are still reachable — the vim
  extension is composed so the clipboard bindings are always installed.
* `Ctrl+Shift+V` (VIM toggle, D9) is deliberately **distinct** from `Ctrl+V` (paste) so the two
  never collide.

**Robustness.** Clipboard access in a webview is subject to browser sandbox rules (on Linux the
`async clipboard` API requires a secure context / user gesture). To make paste dependable on all
platforms the app:

* keeps the native DOM clipboard path as the primary route, and
* falls back to the Tauri clipboard plugin (`tauri-plugin-clipboard-manager`) for
  `write_text` / `read_text`, exposed to the frontend through the `clipboard_*` backend commands.

Both routes are behind one `lib/clipboard.ts` module so the editor code stays platform-agnostic.

---

## 6. Test-driven development strategy

### 6.1 `sandbox-core` (`cargo test`)

| Area | Examples |
|---|---|
| Language detection | `.rs`/`.RS`/`.py`/`.js`/`.mjs` → language; unknown → `None`; `from_path` |
| Command construction | exact argv per language; **no** shell string; code never interpolated into argv |
| Stdin | program that echoes stdin returns it verbatim |
| Streams | stdout and stderr captured separately |
| Exit status | non-zero exit code propagated; `RunStatus::Completed` |
| Compile errors | Rust syntax error → `CompileError`, stderr contains the compiler message, binary not run |
| Timeout | `while true {}` → `TimedOut`, process killed, `duration_ms` bounded |
| Memory | huge allocation → killed by limit |
| Output truncation | output above the cap is truncated and flagged |
| Workspace | temp dir created, isolated per run, removed afterwards |
| Injection | code containing `$(id)`, backticks, `; rm -rf /` is treated as literal text |
| File IO | atomic write → identical content; write → read round-trip; overwriting an existing file works |
| Platform limits | `#[cfg(unix)]` / `#[cfg(windows)]` unit tests for the limit back-ends |

### 6.2 Tauri crate (`cargo test`)

* Command handlers exercised through `tauri::test::mock_builder` / `mock_context`.
* Payload validation and error → serializable-error mapping.

### 6.3 Frontend (`vitest` + `@testing-library/react` + jsdom)

* Layout renders editor + input + output regions.
* Language switch updates the CodeMirror language mode and the default file name.
* Run calls `run_code` with the expected payload; output renders; stderr is visually distinct.
* Auto-run fires exactly once after the debounce window (fake timers).
* `:w` in vim mode triggers **run** and never **save**.
* Vim toggle flips state and the status label.
* Save button opens the dialog and writes to the returned path; the unsaved indicator clears.
* Open loads the file content into the editor.
* Clipboard: the editor keymap contains `Mod-c` / `Mod-v` / `Mod-x` / `Mod-a` bindings; `Mod-c`
  copies the selection (or the whole line when nothing is selected) and `Mod-v` pastes the
  clipboard text at the cursor.
* Clipboard fallback: `lib/clipboard.ts` prefers the DOM clipboard API and falls back to the
  Tauri clipboard commands when the DOM API is unavailable (jsdom / Linux webview restrictions).

### 6.4 CI matrix

`.github/workflows/ci.yml` runs `cargo test` + `npm test` on `ubuntu-latest`, `windows-latest`,
`macos-latest`. This is how the Windows Job Objects code is genuinely **executed** (it cannot run on
this WSL machine; locally it is only compile-checked with
`cargo check --target x86_64-pc-windows-msvc`).

---

## 7. Build order

1. `work.md` + workspace scaffolding.
2. `sandbox-core` test-first until green.
3. Tauri app crate: commands, config, capabilities, icons (compiles once system deps are installed).
4. Frontend test-first until green, wired to the real `invoke` layer.
5. Cross-platform polish: runtime discovery, atomic save, Job Objects, CI matrix.
6. Manual verification in WSLg + README with per-OS instructions.

---

## 8. Platform support

| | Windows | macOS | Linux |
|---|---|---|---|
| Build | `npm run tauri build` → `.msi` / NSIS `.exe` | `.dmg` / `.app` | `.deb` / `.rpm` / AppImage |
| Save dialog | IFileDialog | NSSavePanel | GTK dialog |
| Interpreter discovery | `py -3`, `python`, `node`, `rustc` | `python3`, `node`, `rustc` | `python3`, `node`, `rustc` |
| Resource limits | Job Objects | rlimits | rlimits |
| Clipboard shortcuts | `Ctrl+C` / `Ctrl+V` | `Cmd+C` / `Cmd+V` | `Ctrl+C` / `Ctrl+V` |
| Notes | `CREATE_NO_WINDOW` so no console flashes | — | needs `libwebkit2gtk-4.1-dev` |

---

## 9. Risks & mitigations

| Risk | Mitigation |
|---|---|
| First Tauri build compiles webkit bindings (5–10 min) | incremental afterwards; `cargo test` on `sandbox-core` never needs webkit |
| Windows Job Object code cannot be run on this WSL machine | CI matrix on `windows-latest`; local `cargo check --target x86_64-pc-windows-msvc` |
| WSLg GUI quirks | the editor uses plain DOM APIs; no native drag regions or custom title bar |
| Rust auto-run latency (compile time) | auto-run debounce; compile output streamed to the output panel as it happens |
| User code can damage the machine | documented as *sandbox-like* (limits + temp dir), not a security boundary |

---

## 10. Definition of done

- [ ] `cargo test` green for `sandbox-core` (Linux) and in CI on Windows/macOS.
- [ ] `npm test` green for the frontend.
- [ ] `npm run tauri dev` opens the app: editor (left) + input/output (right).
- [ ] Rust, Python and JavaScript programs run on the fly without saving.
- [ ] Syntax highlighting for all three languages.
- [ ] VIM mode works, `:w` runs the program.
- [ ] Save button opens the native dialog and writes the file; Open reads it back.
- [ ] `Ctrl+C` / `Ctrl+V` (Windows, Linux) and `Cmd+C` / `Cmd+V` (macOS) copy and paste in the editor.
- [ ] `work.md` contains every decision taken, with rationale and date.
