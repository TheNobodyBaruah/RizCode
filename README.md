# Code Sandbox

A standalone, platform-agnostic desktop coding sandbox for **Rust**, **Python**, and **JavaScript**, built with **Tauri v2** (Rust backend) and a **React 19 + TypeScript + CodeMirror 6** frontend.

Code Sandbox provides an immediate, judge-style execution playground: write code, see live output on the fly without saving to disk, test edge cases with custom stdin, and optionally persist your work to a user-chosen path with full OS dialogs.

---

## Table of Contents

- [Overview & Product Goals](#overview--product-goals)
- [Key Design Decisions](#key-design-decisions)
- [Architecture & Layout](#architecture--layout)
  - [The Crate Split (`sandbox-core` vs `app`)](#the-crate-split-sandbox-core-vs-app)
  - [Frontend Architecture](#frontend-architecture)
- [Features in Detail](#features-in-detail)
  - [On-the-Fly Execution Without Saving](#on-the-fly-execution-without-saving)
  - [VIM Mode & `:w` Execution Semantics](#vim-mode--w-execution-semantics)
  - [Explicit Save Dialog & Atomic Writes](#explicit-save-dialog--atomic-writes)
  - [Resource Limits & Platform Parity](#resource-limits--platform-parity)
  - [Cross-Platform Native Clipboard Integration](#cross-platform-native-clipboard-integration)
  - [Window & UI Zoom Scaling](#window--ui-zoom-scaling)
  - [Double-Clickable Executables & Platform Installers](#double-clickable-executables--platform-installers)
- [Platform Support & Prerequisites](#platform-support--prerequisites)
- [Getting Started & Development](#getting-started--development)
  - [Prerequisites](#prerequisites)
  - [Running Tests](#running-tests)
  - [Development Mode](#development-mode)
  - [Building for Production](#building-for-production)
- [Continuous Integration](#continuous-integration)
- [Project Documentation](#project-documentation)
- [License](#license)

---

## Overview & Product Goals

The project satisfies all nine core product goals defined in [plan.md](file:///c:/My_Files/riz_code/plan.md):

| # | Goal | Description & Fulfillment |
|---|------|----------------------------|
| **G1** | **Split IDE Layout** | Two-column desktop layout: main CodeMirror editor pane on the left; user Stdin and separate Stdout/Stderr output consoles stacked vertically on the right with resizable splitter. |
| **G2** | **Syntax Highlighting** | Native syntax modes for **Rust**, **Python**, and **JavaScript** using official CodeMirror 6 language packs (`@codemirror/lang-rust`, `@codemirror/lang-python`, `@codemirror/lang-javascript`). |
| **G3** | **Full VIM Navigation** | Full modal editing powered by `@replit/codemirror-vim`. Toggleable at will; rebinds `:w` to run the program instead of writing to disk. |
| **G4** | **On-the-Fly Execution** | Code is executed purely from memory via ephemeral temporary directories (`code-sandbox-<uuid>`), leaving no residue on user storage. |
| **G5** | **Explicit Permanent Save** | Saving occurs exclusively when the user clicks the toolbar **Save** button, presenting the native OS file picker to select destination folder and filename. |
| **G6** | **Standalone Desktop App** | Self-contained native binary for Windows, macOS, and Linux without external browser or local web server dependencies. |
| **G7** | **Test-Driven Development** | High-coverage test suites for both Rust backend crates (`sandbox-core`, `app`) and the React frontend (`vitest` + `@testing-library/react`). |
| **G8** | **Decision Transparency** | Comprehensive architectural and design decision log recorded in [work.md](file:///c:/My_Files/riz_code/work.md). |
| **G9** | **Native Clipboard Integration** | Full clipboard support inside the editor (`Ctrl+C`/`Ctrl+V`/`Ctrl+X`/`Ctrl+A` on Windows & Linux, `Cmd+C`/`Cmd+V`/`Cmd+X`/`Cmd+A` on macOS) with whole-line copying and native fallback. |
| **G10** | **Window & UI Zoom Scaling** | Keyboard shortcuts (`Ctrl++`/`Ctrl+-`/`Ctrl+0` on Windows/Linux, `Cmd` on macOS) work globally and inside CodeMirror; status/toolbar zoom indicator with interactive +/- controls. |
| **G11** | **Standalone Executables & Installers** | Double-clickable standalone binaries and release installers for Windows (.exe/.msi), macOS (.dmg/.app), and Linux (.AppImage/.deb) uploaded via CI. |

---

## Key Design Decisions

The design choices governing the application are documented in [work.md](file:///c:/My_Files/riz_code/work.md):

- **D1 — In VIM mode, `:w` runs the program**: In Code Sandbox, `:w` serves as quick-action execution ("write to output") and never touches the filesystem.
- **D2 — Saving happens only through the toolbar Save button**: No background auto-save or unexpected file overwrites; saving is always an intentional user act.
- **D3 — Save button opens native OS dialog**: Leverages platform-native dialogs (`IFileDialog` on Windows, `NSSavePanel` on macOS, GTK on Linux) to pick directory and file name.
- **D4 — Full platform parity for resource limits**: Enforces CPU time, wall clock, process counts, and memory limits via `setrlimit` on Unix and **Windows Job Objects** on Windows.
- **D5 — Standalone, platform-agnostic application**: Uniform experience on Windows, macOS, and Linux with zero OS-specific command shell dependencies.
- **D6 — Decoupled `sandbox-core` crate**: Execution engine is completely free of Tauri and WebKit dependencies, running headless `cargo test` in sub-second time.
- **D7 — Direct argv process execution**: Commands execute directly via `std::process::Command` with explicit arguments—never through `sh -c` or `cmd /c`—making shell metacharacter injection impossible.
- **D8 — CodeMirror 6 ecosystem**: Clean bundle, modular extension pipeline, robust Vim emulation, and full testability under jsdom.
- **D9 — VIM mode disabled by default**: Starts in standard editing mode; toggled via toolbar button or `Ctrl+Shift+V` shortcut.
- **D10 — Auto-run on the fly**: 800 ms debounce after typing coalesces edits; sequence numbering drops obsolete runs in flight; `Ctrl+Enter` / `Cmd+Enter` runs immediately.
- **D11 — Rust compiled single-file**: Compiled with `rustc --edition 2021 -o prog main.rs` for fast, reproducible judge-style builds without network access.
- **D12 — Dynamic runtime discovery**: Probes system PATH dynamically for available compilers and interpreters (`rustc`, `python3` / `py -3`, `node`) and warns users if a toolchain is absent.
- **D13 — Atomic file writes**: Saves write first to a temporary file before renaming over the target, preventing corruption on crash or full disk.
- **D14 — Sandbox-like safety boundary**: Designed to catch accidental runaway loops, forks, and memory leaks; not intended as an untrusted multi-tenant security sandbox.
- **D15 — Library vs binary split**: `sandbox-core` is published as a library crate to enable exhaustive integration tests; `app` acts as the Tauri binary.
- **D16 — Centralized frontend store**: Lightweight state management via custom `useSandbox` React hook.
- **D17 — Dedicated plan and decision log**: [plan.md](file:///c:/My_Files/riz_code/plan.md) specifies system design; [work.md](file:///c:/My_Files/riz_code/work.md) tracks chronological rationale.
- **D18 — Platform modifier abstraction (`Mod`)**: Unified keymap using CodeMirror's `Mod` abstraction maps cleanly to `Cmd` on macOS and `Ctrl` on Windows and Linux.
- **D19 — Dual-layer clipboard fallback**: Primary route uses the DOM Clipboard API; falls back seamlessly to Tauri's native `tauri-plugin-clipboard-manager` when webview permissions are restricted.
- **D20 — Matrix CI across Ubuntu, Windows, and macOS**: Validates `cargo test` and `npm test` across all three major platforms, ensuring genuine execution of Windows Job Objects.
- **D21 — Unified multi-platform onboarding and documentation**: Comprehensive guides covering all prerequisites, libraries, and workflows across Windows, macOS, and Linux.
- **D22 — Window & UI zoom scaling semantics**: Discrete 70% to 200% zoom scaling, CodeMirror keymap delegation, CSS root scaling, and interactive indicator controls.
- **D23 — Standalone double-clickable executables & installers**: Tauri v2 bundling configured for Windows (NSIS .exe, .msi, standalone .exe), Linux (.deb, .AppImage, standalone binary), and macOS (.dmg, .app) with CI artifact distribution.

---

## Architecture & Layout

```
code_sandbox/
├── Cargo.toml                       # Workspace root definition
├── plan.md                          # Architectural specification
├── work.md                          # Chronological decision log
├── .github/
│   └── workflows/
│       └── ci.yml                   # Matrix CI (Ubuntu, Windows, macOS)
├── crates/
│   ├── sandbox-core/                # Pure Rust execution engine (Zero Tauri deps)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs               # Public API exports
│   │   │   ├── language.rs          # Language detection, file names, compiler args
│   │   │   ├── limits.rs            # Execution limit configuration & defaults
│   │   │   ├── runner.rs            # Compilation, execution, stream capture
│   │   │   ├── runtime.rs           # System interpreter & compiler discovery
│   │   │   ├── workspace.rs         # Ephemeral temp directory lifecycle
│   │   │   ├── fs.rs                # Atomic file write & read utilities
│   │   │   └── platform/
│   │   │       ├── mod.rs           # Platform dispatch
│   │   │       ├── unix.rs          # Linux/macOS rlimits & process groups
│   │   │       └── windows.rs       # Windows Job Objects & memory limits
│   │   └── tests/                   # Integration tests with real compilers
│   └── app/                         # Tauri v2 desktop shell
│       ├── Cargo.toml
│       ├── tauri.conf.json          # Tauri configuration & window settings
│       ├── capabilities/            # Security capabilities (dialog, clipboard)
│       └── src/
│           ├── main.rs              # App entry point
│           ├── lib.rs               # Tauri builder & plugin wiring
│           ├── commands.rs          # Tauri command handlers
│           └── state.rs             # Shared backend state
└── frontend/                        # React 19 + TypeScript + Vite UI
    ├── package.json
    ├── vite.config.ts
    ├── tsconfig.json
    └── src/
        ├── App.tsx                  # Two-column layout container
        ├── components/
        │   ├── Editor.tsx           # CodeMirror 6 editor wrapper
        │   ├── Toolbar.tsx          # Language picker, Save, Run, Vim toggle
        │   ├── InputPanel.tsx       # User Stdin text area
        │   ├── OutputPanel.tsx      # Stdout & Stderr views
        │   └── StatusBar.tsx        # Runtime info, Vim mode indicator, exit codes
        ├── lib/
        │   ├── api.ts               # Tauri invoke wrappers
        │   ├── clipboard.ts         # Dual-layer clipboard service
        │   ├── keymap.ts            # Mod-based shortcuts
        │   ├── languages.ts         # Language modes & sample snippets
        │   ├── theme.ts             # Editor theme configuration
        │   └── vim.ts               # Vim initialization & :w command hook
        └── store/
            └── useSandbox.ts        # Central state hook
```

### The Crate Split (`sandbox-core` vs `app`)

```
┌────────────────────────────────────────────────────────┐
│               Frontend (React + Vite)                  │
└───────────────────────────┬────────────────────────────┘
                            │ Tauri IPC invoke
┌───────────────────────────▼────────────────────────────┐
│                crates/app (Tauri Shell)                │
│  - commands.rs (run_code, save_file, clipboard_*, ...) │
└───────────────────────────┬────────────────────────────┘
                            │ Rust API call
┌───────────────────────────▼────────────────────────────┐
│               crates/sandbox-core                      │
│  ├─ workspace.rs: Throwaway temp directories           │
│  ├─ runtime.rs: Interpreter discovery (rustc/py/node)  │
│  ├─ runner.rs: Child process spawn, stdin/stdout/err   │
│  ├─ limits.rs: Timeout, memory, output cap configs     │
│  ├─ platform/unix.rs: setrlimit & POSIX process groups │
│  └─ platform/windows.rs: Windows Job Objects           │
└────────────────────────────────────────────────────────┘
```

By decoupling `sandbox-core` from Tauri, the entire compilation and execution pipeline is thoroughly tested via `cargo test` on bare headless servers and CI machines without requiring display servers, GTK, or WebKit dependencies.

---

## Features in Detail

### On-the-Fly Execution Without Saving

- **Zero Disk Footprint**: Code is edited directly in memory. When executing, `sandbox-core` provisions an isolated directory in `std::env::temp_dir()/code-sandbox-<uuid>`, writes the source file, compiles or interprets it, collects stdout/stderr, and unconditionally removes the directory when the run completes.
- **Live Debounced Execution**: Changes in the editor trigger a background run after an 800 ms debounce delay, giving immediate feedback without triggering process storms.
- **Sequence Numbering**: Every run request is tagged with an incremental sequence ID. If a newer run finishes after an older slow run, the older result is automatically discarded to prevent UI state regression.
- **Immediate Execution**: Pressing `Ctrl+Enter` (Windows/Linux) or `Cmd+Enter` (macOS) triggers immediate execution, bypassing the debounce window.

### VIM Mode & `:w` Execution Semantics

- **Modal Editing**: Powered by `@replit/codemirror-vim`, offering standard normal, insert, visual, and ex-command modes.
- **Custom `:w` Binding**: In traditional editors, `:w` writes the file to disk. In Code Sandbox, `:w` is mapped to **run the active code**. It transmits the buffer to the sandbox runner and streams output to the console pane—**it never writes to disk**.
- **Mode Indication**: The status bar displays the active mode (`NORMAL`, `INSERT`, `VISUAL`) alongside a prominent reminder: `VIM: :w runs code`.
- **Quick Toggle**: VIM mode is disabled by default and can be toggled via the toolbar switch or by pressing `Ctrl+Shift+V` at any time.

### Explicit Save Dialog & Atomic Writes

- **Deliberate Persistence**: The only mechanism for saving code permanently to your filesystem is clicking the **Save** button in the header toolbar.
- **Native File Picker**: Code Sandbox invokes the operating system's native file dialog (`tauri-plugin-dialog`), allowing the user to select any directory and enter an arbitrary file name.
- **Atomic File Writes**: File saving uses atomic write semantics—writing to an adjacent `.tmp` file before renaming it over the destination target. This guarantees that destination files are never corrupted or left half-written.
- **Dirty State Tracking**: An indicator dot appears beside the active filename whenever in-memory buffer edits diverge from the saved file on disk.

### Resource Limits & Platform Parity

Code Sandbox enforces rigorous execution bounds across all supported platforms:

| Resource Limit | Linux / macOS (`setrlimit`) | Windows (`Job Objects`) | Default Threshold |
|---|---|---|---|
| **Wall-Clock Timeout** | Watchdog thread + `kill(-pgid, SIGKILL)` | Watchdog thread + `TerminateJobObject` | 5 seconds |
| **CPU Time** | `RLIMIT_CPU` | `JOB_OBJECT_LIMIT_PROCESS_TIME` | 5 seconds |
| **Virtual Memory** | `RLIMIT_AS` | `JOB_OBJECT_LIMIT_PROCESS_MEMORY` | 512 MiB |
| **Process Count** | `RLIMIT_NPROC` | `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` | 64 processes |
| **Output Size Cap** | In-memory capped stream reader | In-memory capped stream reader | 1 MiB |
| **Child Cleanup** | Process group kill (`setpgid`) | `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` | Cleaned on exit |

> **Note on Security (D14)**: Code Sandbox limits are engineered to protect against accidental infinite loops, exponential recursion, fork bombs, and excessive memory allocations. It runs with user-level privileges and does not constitute a virtualized hypervisor sandbox.

### Cross-Platform Native Clipboard Integration

- **Modifier Key Awareness**: The editor configures keymaps using CodeMirror's `Mod` key abstraction:
  - **Windows & Linux**: `Ctrl+C` (Copy), `Ctrl+V` (Paste), `Ctrl+X` (Cut), `Ctrl+A` (Select All)
  - **macOS**: `Cmd+C` (Copy), `Cmd+V` (Paste), `Cmd+X` (Cut), `Cmd+A` (Select All)
- **Line Copying**: When no text is selected, pressing `Ctrl+C` or `Cmd+C` copies the entire current line, including the line break.
- **VIM Coexistence**: While VIM mode is active, native clipboard commands remain fully functional in addition to standard VIM yanking (`y`) and putting (`p`).
- **Resilient Fallback**: If the webview's DOM `navigator.clipboard` API is restricted by platform permissions or headless test runners, the app transparently delegates to the backend Tauri clipboard plugin (`tauri-plugin-clipboard-manager`).

### Window & UI Zoom Scaling

- **Zoom Shortcuts**:
  - **Zoom In**: `Ctrl + +` or `Ctrl + =` (Windows/Linux) / `Cmd + +` or `Cmd + =` (macOS)
  - **Zoom Out**: `Ctrl + -` (Windows/Linux) / `Cmd + -` (macOS)
  - **Reset Zoom**: `Ctrl + 0` (Windows/Linux) / `Cmd + 0` (macOS)
- **Universal Event Routing**: Zoom shortcuts work both inside CodeMirror editor (via `lib/keymap.ts`) and globally across the entire UI window.
- **Proportional UI Scaling**: Dynamic CSS scaling uniformly adjusts editor fonts, toolbar controls, stdin/stdout panels, and status bar elements (range: 70% to 200% in 10% increments).
- **Interactive Controls**: A dedicated zoom percentage badge in the toolbar and status bar provides one-click `+`, `-`, and `Reset` buttons.

### Double-Clickable Executables & Platform Installers

Code Sandbox produces standalone double-clickable executables and standard installer packages for easy distribution:
- **Windows**: Self-contained double-clickable `code-sandbox-app.exe`, plus guided NSIS setup installer (`.exe`) and MSI package (`.msi`).
- **Linux**: Standalone executable, portable double-clickable `.AppImage` (bundled with desktop icon and entry), and `.deb` package.
- **macOS**: Drag-and-drop `.dmg` installer disk image and native `.app` application bundle.
- **Automated CI Releases**: The GitHub Actions matrix workflow builds release packages on every runner and uploads them as downloadable artifacts.

---

## Platform Support & Prerequisites

| Platform | Compiler / Toolchain | Packaging Target | Native Dialogs | Resource Backend | Zoom Shortcuts |
|---|---|---|---|---|---|
| **Windows 10/11** | MSVC (x86_64) | NSIS `.exe` / `.msi` / standalone `.exe` | `IFileDialog` | Windows Job Objects | `Ctrl++`, `Ctrl+-`, `Ctrl+0` |
| **macOS 12+** | Apple Clang / Xcode | `.dmg` / `.app` bundle / standalone binary | `NSSavePanel` | POSIX `setrlimit` | `Cmd++`, `Cmd+-`, `Cmd+0` |
| **Linux (Ubuntu/Fedora/Arch)** | GCC / Clang | `.deb` / `.AppImage` / standalone binary | GTK FileChooser | POSIX `setrlimit` | `Ctrl++`, `Ctrl+-`, `Ctrl+0` |

---

## Getting Started & Development

### Prerequisites

1. **Rust Toolchain**: Install Rust stable (1.77+):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. **Node.js**: Install Node.js (v20+ or v22+) and `npm`:
   ```bash
   # via nvm
   nvm install 22
   nvm use 22
   ```
3. **Execution Runtimes** (Ensure target languages are installed and available in your `PATH`):
   - **Rust**: `rustc` (`rustc --version`)
   - **Python**: `python3` (or `py -3` on Windows)
   - **JavaScript**: `node` (`node --version`)
4. **Linux-Specific Dependencies**:
   On Ubuntu / Debian systems, install the required GTK and WebKit development libraries:
   ```bash
   sudo apt-get update
   sudo apt-get install -y \
     build-essential \
     curl \
     wget \
     file \
     libssl-dev \
     libgtk-3-dev \
     libwebkit2gtk-4.1-dev \
     libappindicator3-dev \
     librsvg2-dev \
     patchelf
   ```

### Running Tests

Execute the complete test suite across all layers:

```bash
# 1. Run all Rust unit and integration tests (sandbox-core & app)
cargo test --workspace

# 2. Run Rust tests for sandbox-core only
cargo test -p sandbox-core

# 3. Run frontend unit and component tests
cd frontend
npm test

# 4. Check frontend TypeScript types and Vite production build
npm run build
```

### Development Mode

Run the desktop application in live development mode with hot module reloading (HMR):

```bash
cd frontend
npm run tauri dev
```

To run only the React frontend in a web browser (with mocked Tauri backend commands for UI styling):

```bash
cd frontend
npm run dev
```

### Building for Production

Compile optimized, self-contained desktop binaries and platform installers:

```bash
cd frontend
npm run tauri build
```

Installers and binaries will be generated in:
`crates/app/target/release/bundle/`

---

## Continuous Integration

Code Sandbox utilizes GitHub Actions to test on every pull request and push to `main` across all target operating systems.

See [.github/workflows/ci.yml](file:///c:/My_Files/riz_code/.github/workflows/ci.yml) for the full matrix definition:
- **Ubuntu Latest**: Executes `cargo test`, `npm test`, and Linux WebKit compilation checks.
- **Windows Latest**: Compiles and executes Windows Job Object limits (`crates/sandbox-core/src/platform/windows.rs`).
- **macOS Latest**: Validates POSIX limit enforcement and macOS clipboard shortcuts.

---

## Project Documentation

- [plan.md](file:///c:/My_Files/riz_code/plan.md): Complete engineering plan, layout blueprints, and verification criteria.
- [work.md](file:///c:/My_Files/riz_code/work.md): Chronological record of design choices, trade-offs, and technical rationales (D0–D19).
- [.github/workflows/ci.yml](file:///c:/My_Files/riz_code/.github/workflows/ci.yml): Multi-platform CI pipeline definition.

---

## License

This project is licensed under the [MIT License](file:///c:/My_Files/riz_code/LICENSE).
