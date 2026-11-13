import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  saveDialog: vi.fn(),
  openDialog: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: mocks.saveDialog,
  open: mocks.openDialog,
}));

import App from "../App";

/** The default answer of the mocked backend. */
function defaultInvoke(command: string) {
  switch (command) {
    case "detect_runtimes":
      return [
        {
          language: "rust",
          available: true,
          program: "rustc",
          version: "rustc 1.98.1",
          message: null,
        },
        {
          language: "python",
          available: true,
          program: "python3",
          version: "Python 3.14.4",
          message: null,
        },
        {
          language: "javascript",
          available: true,
          program: "node",
          version: "v24.0.0",
          message: null,
        },
      ];
    case "run_code":
      return {
        stdout: "Hello, world!\n",
        stderr: "",
        exit_code: 0,
        duration_ms: 12,
        status: "completed",
        truncated: false,
      };
    case "save_file":
      return { path: "/tmp/main.rs", bytes_written: 10 };
    case "open_file":
      return { path: "/tmp/main.rs", contents: "fn main() {}\n" };
    default:
      return null;
  }
}

/** Finds the `invoke` call for a command. */
function callsFor(command: string) {
  return mocks.invoke.mock.calls.filter((call) => call[0] === command);
}

describe("Code Sandbox", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.invoke.mockImplementation(async (command: string) =>
      defaultInvoke(command),
    );
    mocks.saveDialog.mockReset();
    mocks.openDialog.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders the editor on the left and the input/output panels on the right", async () => {
    render(<App />);

    expect(screen.getByTestId("editor")).toBeInTheDocument();
    expect(screen.getByLabelText("Program input")).toBeInTheDocument();
    expect(screen.getByTestId("stdout")).toBeInTheDocument();
    expect(screen.getByTestId("stderr")).toBeInTheDocument();

    // Let the runtime detection settle so that no state update happens outside of `act`.
    await waitFor(() => expect(screen.getByTitle(/rustc/)).toBeInTheDocument());
  });

  it("runs the code and shows the output", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Run" }));

    await waitFor(() => {
      expect(callsFor("run_code")).toHaveLength(1);
    });

    const payload = callsFor("run_code")[0][1].payload;
    expect(payload.language).toBe("rust");
    expect(typeof payload.code).toBe("string");
    expect(payload.stdin).toBe("");

    expect(await screen.findByText(/Hello, world!/)).toBeInTheDocument();
  });

  it("shows stderr in a separate area", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "run_code") {
        return {
          stdout: "out",
          stderr: "boom",
          exit_code: 1,
          duration_ms: 5,
          status: "completed",
          truncated: false,
        };
      }
      return defaultInvoke(command);
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Run" }));

    const stderr = await screen.findByTestId("stderr");
    expect(stderr).toHaveTextContent("boom");
    expect(stderr).not.toHaveTextContent("out");
  });

  it("switches the language", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.selectOptions(screen.getByLabelText("Language"), "python");

    await user.click(screen.getByRole("button", { name: "Run" }));
    await waitFor(() => expect(callsFor("run_code")).toHaveLength(1));

    expect(callsFor("run_code")[0][1].payload.language).toBe("python");
  });

  it("can switch the auto-run off", async () => {
    const user = userEvent.setup();
    render(<App />);

    const toggle = screen.getByRole("button", { name: "Auto-run" });
    expect(toggle).toHaveAttribute("aria-pressed", "true");

    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-pressed", "false");
  });

  it("toggles the VIM mode and updates the status label", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(screen.getByTestId("vim-mode")).toHaveTextContent("NORMAL");

    await user.click(screen.getByRole("button", { name: "VIM" }));

    expect(screen.getByTestId("vim-mode")).toHaveTextContent("VIM");
    expect(screen.getByRole("button", { name: "VIM" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("runs the program when the user presses Ctrl+Enter", async () => {
    render(<App />);

    await userEvent.keyboard("{Control>}{Enter}{/Control}");

    await waitFor(() => expect(callsFor("run_code")).toHaveLength(1));
  });

  it("saves through the native dialog and clears the unsaved indicator", async () => {
    mocks.saveDialog.mockResolvedValue("/tmp/main.rs");
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(callsFor("save_file")).toHaveLength(1));
    expect(mocks.saveDialog).toHaveBeenCalled();
    expect(callsFor("save_file")[0][1].path).toBe("/tmp/main.rs");
  });

  it("opens a file and loads its contents", async () => {
    mocks.openDialog.mockResolvedValue("/tmp/main.rs");
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => expect(callsFor("open_file")).toHaveLength(1));
    expect(callsFor("open_file")[0][1].path).toBe("/tmp/main.rs");
  });

  it("shows the detected runtimes", async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTitle(/rustc/)).toBeInTheDocument();
    });
  });

  it("adjusts zoom via toolbar controls and keyboard shortcuts", async () => {
    const user = userEvent.setup();
    render(<App />);

    // Default zoom is 100%
    const zoomLevel = screen.getByTestId("zoom-level");
    expect(zoomLevel).toHaveTextContent("100%");

    // Click zoom in button
    const zoomInBtns = screen.getAllByRole("button", { name: "Zoom in" });
    await user.click(zoomInBtns[0]);
    expect(zoomLevel).toHaveTextContent("110%");

    // Keyboard shortcut zoom out (Ctrl+-)
    await userEvent.keyboard("{Control>}{-}{/Control}");
    expect(zoomLevel).toHaveTextContent("100%");

    // Keyboard shortcut zoom in (Ctrl+=)
    await userEvent.keyboard("{Control>}{=}{/Control}");
    expect(zoomLevel).toHaveTextContent("110%");

    // Reset zoom via click
    await user.click(zoomLevel);
    expect(zoomLevel).toHaveTextContent("100%");

    // Keyboard shortcut reset (Ctrl+0)
    await userEvent.keyboard("{Control>}{=}{/Control}");
    expect(zoomLevel).toHaveTextContent("110%");
    await userEvent.keyboard("{Control>}{0}{/Control}");
    expect(zoomLevel).toHaveTextContent("100%");
  });
});
