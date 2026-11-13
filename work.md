# Decision Log — Code Sandbox

Every decision taken while building this project, with the reasoning behind it.
Newest decisions are appended at the bottom.

---

## 2026-11-13 — Project kickoff

### D0. Stack: Tauri v2 + Rust backend, React/TypeScript frontend

**Decision.** The desktop shell is [Tauri v2](https://tauri.app) (Rust), the UI is React 19 +
TypeScript + Vite, the editor is CodeMirror 6.

**Why.** The user asked for "a desktop application using the Rust Programming Language and Tauri".
Tauri v2 gives a small, fast, dependency-light binary and, importantly, puts all privileged
operations (process spawning, file IO) inside Rust, which is exactly where we want the testable
core to live.

**Alternatives considered.** Electron (huge binary, Node process in the middle of everything),
native widgets (Qt/Iced — much more work for a code editor with syntax highlighting and vim mode).

---

### D1. In VIM mode `:w` runs the program instead of saving

**Decision.** The vim ex-command `:w` is remapped to **run** the code. It never writes a file.

**Why.** Direct user requirement: *"` :w ` command shouldn't run the file rather it should run the
file"* — i.e. in this app `:w` is a shorthand for "write my work to the output", which the user
wants to mean *execute*. The user explicitly confirmed this reading in the follow-up question.

**Consequence.** This deliberately deviates from stock vim semantics, so it is documented in the
README, in the in-app status bar hint, and here. Saving is reachable only through the toolbar
button (see D2), which keeps exactly one code path that writes to disk.

---

### D2. Saving happens only through a dedicated toolbar Save button

**Decision.** No keyboard shortcut, no autosave, no "save on exit". The only way to write the
buffer to disk is the **Save** button in the toolbar.

**Why.** The user's brief distinguishes between "get the output on the fly without saving the file
permanently" and "if they wish they could save the file permanently in a desired path". Keeping a
single, explicit, user-initiated write path makes the app predictable and auditable: nothing on
disk changes unless the user asked for it.

---

### D3. The Save button opens the native OS save dialog (folder + filename)

**Decision.** Clicking **Save** opens the platform-native save panel where the user picks a
directory *and* types a filename. No custom "type a path" text box.

**Why.** Native dialogs are familiar on all three platforms (IFileDialog on Windows, NSSavePanel on
macOS, GTK on Linux), support bookmarks/recent places, and are what users expect. A hand-rolled
path field is error-prone (separator styles, drive letters, `~` expansion).

**Details.** The dialog is pre-filled with the last used directory and the language's default
file name (`main.rs`, `main.py`, `main.js`). Implementation uses `tauri-plugin-dialog` from the
frontend; the actual write happens in the Rust backend so it stays unit-testable (D13).

---

### D4. Full platform parity for resource limits (Windows Job Objects)

**Decision.** Memory, CPU, process-count and file-size limits are enforced on **all** platforms:
`setrlimit` on Linux/macOS, **Windows Job Objects** on Windows.

**Why.** The user asked for a platform-agnostic app. Behaviour that differs per platform (e.g.
"timeouts everywhere but memory limits only on Linux") would be a portability bug for users who
move between machines. The user chose the parity option explicitly.

**Consequence.** Windows-specific code cannot be executed on this WSL/Linux development machine.
It is therefore guarded with `#[cfg(windows)]` and covered by a CI job on `windows-latest`
(see D5); locally it is compile-checked with `cargo check --target x86_64-pc-windows-msvc`.

---

### D5. Standalone, platform-agnostic application (Windows / macOS / Linux)

**Decision.** One codebase, three native targets, each producing a self-contained desktop binary
that needs no browser and no local server.

**Why.** Direct user requirement: *"It should be standalone application and should be platform
agnostic, which means it should be able to run on windows, mac and linux."*

**Details.** Concretely this drove: no hardcoded interpreter paths (D12), no POSIX-only shell
invocation (D7), atomic writes that work on Windows (D13), `CREATE_NO_WINDOW` on Windows so no
console window flashes, `Ctrl`/`Cmd` abstraction in the frontend, and a CI matrix over the three
operating systems.

---

### D6. All logic lives in a Tauri-free `sandbox-core` crate

**Decision.** The workspace has two crates:

* `crates/sandbox-core` — pure Rust, **zero** Tauri/webkit dependencies: language detection,
  compilation/execution, limits, temp-workspace lifecycle, atomic file IO.
* `crates/app` — the Tauri crate: a thin adapter exposing `run_code`, `save_file`, `open_file`,
  `detect_runtimes`, `app_info`.

**Why.** The user asked for TDD. If the execution engine lived inside the Tauri crate, every test
would need a webview/webkit toolchain and a display server, which makes the test cycle slow and
fragile. Splitting it out means `cargo test` on the risky part runs in ~1 second anywhere — in
this container, in CI, on a colleague's laptop.

**Consequence.** The Tauri crate stays intentionally boring: it converts serde DTOs into core
calls and back.

---

### D7. Programs are spawned directly with argv, never through a shell

**Decision.** `std::process::Command::new(program).args([...])`. No `sh -c`, no `cmd /C`, no string
interpolation of user code into a command line.

**Why.** Spawning through a shell would mean user code could be interpreted as shell syntax
(`$(id)`, backticks, `; rm -rf /`). Direct argv spawning makes code content inert. It is also
faster (no shell process) and avoids Windows quoting nightmares.

**Test.** `sandbox-core` has an explicit test asserting that code containing shell metacharacters
is executed literally and cannot escape.

---

### D8. CodeMirror 6 (not Monaco, not Ace)

**Decision.** Editor: CodeMirror 6 with `@codemirror/lang-rust`, `@codemirror/lang-python`,
`@codemirror/lang-javascript`.

**Why.** The user asked for syntax colouring **and** VIM mode. CodeMirror 6 has a first-class,
maintained vim mode (`@replit/codemirror-vim`), a small bundle, and excellent testability.
Monaco is VS Code's editor: heavier, with no mature vim mode, and painful in jsdom.
Ace is lighter but weaker on language support and ecosystem.

---

### D9. VIM mode is off by default

**Decision.** The editor starts in normal mode; VIM is toggled from the toolbar or with
`Ctrl+Shift+V`.

**Why.** Vim mode is a power-user feature. Enabling it by default would break normal editing for
users who just want to type code — and the user's brief lists it as a capability, not a default.

**Note.** `Ctrl+V` is *not* used as the toggle because it is visual-mode inside vim; `Ctrl+Shift+V`
is used instead.

---

### D10. Auto-run on the fly: 800 ms debounce + `Ctrl+Enter`

**Decision.** After the last keystroke, the app waits 800 ms and then runs the code. A toolbar
toggle disables this. `Ctrl+Enter` runs immediately regardless.

**Why.** The user asked to "type the code and get the output on the fly without saving the file
permanently". A debounce is required: without it, every keystroke would spawn a process
storm. 800 ms is long enough to coalesce a burst of typing but short enough to feel live.

**Details.** Runs are sequence-numbered: if a new run starts while another is in flight, the older
one's result is discarded so the output panel never shows stale results. The Rust compile step is
the slow part; compilation messages are streamed to the output panel as they arrive.

---

### D11. Rust is compiled with `rustc --edition 2021`, single file

**Decision.** `rustc --edition 2021 -o prog main.rs`, then run the produced binary.

**Why.** The user chose the "rustc single-file, no crates" option, matching the single-file,
judge-style workflow of the web sandboxes the app is modelled on (CodeChef et al.). `cargo` would
also mean a network/cache dependency and multi-second-plus builds for a feature the brief does
not ask for.

**Details.** Edition 2021 rather than 2024 for wider toolchain compatibility; it is a single
constant in `language.rs` and trivial to change.

---

### D12. Interpreters are discovered at runtime, never hardcoded

**Decision.** The backend probes for each toolchain and reports availability + version:

| Language | Probe order |
|---|---|
| Rust | `rustc --version` |
| Python | Unix: `python3`, `python` · Windows: `py -3`, `python`, `python3` |
| JavaScript | `node`, `nodejs` |

The UI shows a badge when a language's runtime is missing.

**Why.** Hardcoding `python3` breaks on Windows (where it is usually `python` or the `py` launcher)
and on some Linux distributions (Debian ships `python3` but Fedora sometimes only offers
`python3.11`). Discovery keeps the app platform-agnostic (D5) and lets the user see *why* a
language fails instead of getting a cryptic spawn error.

---

### D13. Source files are written atomically

**Decision.** `save_file` writes to a temporary file in the destination directory and then renames
it over the target (Windows: remove-then-rename, since `std::fs::rename` cannot replace an
existing file there).

**Why.** A crash or a full disk during a direct write leaves a truncated, unparsable source file.
The temp+rename pattern makes the save all-or-nothing.

**Related.** Line endings are preserved exactly as typed (no CRLF↔LF conversion) and files are
read/written as UTF-8; opening a non-UTF-8 file produces a clear error rather than mojibake.

---

### D14. The app is *sandbox-like*, not a security boundary

**Decision.** Documented explicitly in the README: the runner applies time, memory, process and
file-size limits and isolates each run in a throwaway temp directory, but the code still runs
with the user's own privileges. It is a convenience sandbox for one's own code, not a
multi-tenant or untrusted-code environment.

**Why.** Being honest about this matters: a desktop app that runs arbitrary code cannot claim
kernel-level isolation without containers/seccomp/jail, which is out of scope for a desktop IDE.
The limits still prevent the classic accidents (infinite loops, runaway allocations, fork bombs).

---

### D15. `sandbox-core` is a library crate; the Tauri app is a binary crate

**Decision.** `crates/sandbox-core` is a `lib` crate; `crates/app` is a `bin` crate (plus a small
`lib` target so integration tests can reach its internals).

**Why.** Integration tests in `crates/sandbox-core/tests/` need to import the crate under test —
a binary crate cannot be imported by another test target. This also keeps the public API of the
core deliberate.

---

### D16. Frontend state lives in a single `useSandbox` store

**Decision.** A React hook (`store/useSandbox.ts`) owns language, buffer, stdin, output, run status
and the save path; components consume it through props/hooks.

**Why.** Avoids pulling in a state library for what is a small, well-defined state machine, and
keeps components trivially testable with `@testing-library/react` (no store mocking needed).
A `zustand`-free design also means the frontend test suite needs no extra dependencies.

---

### D17. The `plan.md` file is kept in the repository root

**Decision.** The full implementation plan lives in `plan.md` at the repo root, next to the
runtime decision log `work.md`.

**Why.** The user asked for the plan to be written into `plan.md` and for every decision to be
recorded in `work.md`. Keeping the plan (static design) and the log (chronological record) in
separate files avoids the log being rewritten every time the design is touched.

---

## 2026-11-13 — Clipboard support (continuation session)

### D18. Clipboard uses the platform-native modifier (`Mod`)

**Decision.** Copy/paste are bound to CodeMirror's `Mod` modifier: `Mod-c` / `Mod-v` / `Mod-x` /
`Mod-a`. `Mod` resolves to **Cmd** on macOS and to **Ctrl** on Windows and Linux, so:

| OS | Copy | Paste |
|---|---|---|
| Windows | `Ctrl+C` | `Ctrl+V` |
| Linux | `Ctrl+C` | `Ctrl+V` |
| macOS | `Cmd+C` | `Cmd+V` |

The bindings are declared in an **explicit application keymap** (`frontend/src/lib/keymap.ts`)
rather than being left to `basicSetup`, and they are always composed *after* the VIM extension so
they stay reachable while VIM mode is active.

**Why.** Direct user requirement: *"it should allow copying to the clipboard with `Ctrl + C` and
`Ctrl + V` for windows and `Cmd + C` and `Cmd + V` for mac and also the linux respective
commands."*

**Details.** Writing the binding in terms of `Mod` instead of raw `Ctrl` is what makes this
platform-agnostic in the sense of D5: one implementation, three correct behaviours, and **no OS
sniffing** in the frontend. `Ctrl+Shift+V` (the VIM toggle, D9) is kept distinct from `Ctrl+V`
(paste) so the two can never collide.

**Semantics.** With a selection, `Mod-c` copies the selection and `Mod-x` cuts it. With an empty
selection, `Mod-c` copies the whole current line and `Mod-v` inserts the clipboard at the cursor —
the behaviour of every mainstream editor, and the behaviour a user expects from "select all, copy,
paste" workflows.

### D19. Clipboard has a native (Tauri) fallback route

**Decision.** All clipboard access goes through `frontend/src/lib/clipboard.ts`, which:

1. tries the DOM `navigator.clipboard` API, and
2. falls back to the `clipboard_write_text` / `clipboard_read_text` Tauri commands, backed by
   `tauri-plugin-clipboard-manager`.

**Why.** In a webview the async Clipboard API is not universally available: on Linux it requires a
secure context and a user gesture, and in some WebKitGTK builds `navigator.clipboard` is missing
entirely. Without a fallback, paste would silently do nothing on exactly the platform (Linux) where
the user explicitly asked for `Ctrl+V` to work. The fallback also makes the clipboard code
testable under jsdom, where `navigator.clipboard` does not exist.

**Consequence.** The editor keymap calls the *same* `lib/clipboard.ts` helpers that the DOM
fallback uses, so there is exactly one place where clipboard semantics live.

---

## 2026-11-13 — Infrastructure, CI matrix & documentation

### D20. Matrix CI across Ubuntu, Windows, and macOS

**Decision.** A GitHub Actions workflow (`.github/workflows/ci.yml`) runs the full test suite
(`cargo test --workspace` and `npm test` + `npm run build` in `frontend/`) on `ubuntu-latest`,
`windows-latest`, and `macos-latest` on every push and pull request to `main`.

**Why.** Fulfills D4 and D5: the Windows Job Object resource limits cannot be run natively on a
Linux/WSL host; the CI matrix ensures that the Windows Job Object implementation is genuinely
compiled and exercised on Windows runner kernels, alongside POSIX `setrlimit` on Ubuntu and macOS.
It also ensures the WebKit/GTK system dependencies compile cleanly on Linux runners.

---

### D21. Unified multi-platform onboarding and documentation

**Decision.** A comprehensive `README.md` details all architectural layers, product goals (G1–G9),
design choices (D0–D20), resource limit semantics, VIM `:w` execution behaviour, and explicit
step-by-step setup, test, and build instructions for Windows, macOS, and Linux.

**Why.** A standalone multi-platform application requires explicit documentation for prerequisites
(runtimes like `rustc`, `python3`/`py`, `node`), OS-specific system libraries (e.g. `libwebkit2gtk-4.1-dev`
on Ubuntu/Debian), and development workflows. Tying the documentation directly to `plan.md` and
`work.md` maintains project transparency (G8).

