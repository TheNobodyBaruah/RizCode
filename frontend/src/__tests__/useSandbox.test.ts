import { describe, expect, it, vi, beforeEach } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

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

import { useSandbox, AUTO_RUN_DEBOUNCE_MS } from "../store/useSandbox";

/** A promise that can be resolved from the outside. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

/** Renders the store and lets the runtime detection settle inside `act`. */
async function renderStore() {
  const view = renderHook(() => useSandbox());
  await waitFor(() => expect(view.result.current.runtimes).toEqual([]));
  return view;
}

/** Builds a run outcome. */
function outcome(stdout: string) {
  return {
    stdout,
    stderr: "",
    exit_code: 0,
    duration_ms: 1,
    status: "completed" as const,
    truncated: false,
  };
}

describe("the sandbox store", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "detect_runtimes") return [];
      return outcome("");
    });
  });

  it("starts with the Rust starter program", async () => {
    const { result } = await renderStore();
    expect(result.current.language).toBe("rust");
    expect(result.current.code).toContain("fn main()");
  });

  it("marks the buffer as dirty when it changes", async () => {
    const { result } = await renderStore();

    act(() => {
      result.current.setCode("print('hi')");
    });

    expect(result.current.code).toBe("print('hi')");
    expect(result.current.dirty).toBe(true);
  });

  it("discards the result of a superseded run", async () => {
    const first = deferred<ReturnType<typeof outcome>>();
    const second = deferred<ReturnType<typeof outcome>>();
    let call = 0;

    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "run_code") {
        call += 1;
        return call === 1 ? first.promise : second.promise;
      }
      return [];
    });

    const { result } = await renderStore();

    act(() => {
      void result.current.run();
    });
    act(() => {
      void result.current.run();
    });

    await act(async () => {
      second.resolve(outcome("second"));
      await Promise.resolve();
    });

    await act(async () => {
      first.resolve(outcome("first"));
      await Promise.resolve();
    });

    await waitFor(() => expect(result.current.stdout).toBe("second"));
  });

  it("saves the file to the path the dialog returned", async () => {
    mocks.saveDialog.mockResolvedValue("/tmp/out.py");
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "save_file") {
        return { path: "/tmp/out.py", bytes_written: 3 };
      }
      return [];
    });

    const { result } = await renderStore();

    let saved: string | null = null;
    await act(async () => {
      saved = await result.current.save();
    });

    expect(mocks.saveDialog).toHaveBeenCalled();
    expect(saved).toBe("/tmp/out.py");
    expect(result.current.filePath).toBe("/tmp/out.py");
    expect(result.current.dirty).toBe(false);
  });

  it("does nothing when the save dialog is cancelled", async () => {
    mocks.saveDialog.mockResolvedValue(null);

    const { result } = await renderStore();

    let saved: string | null = "not-null";
    await act(async () => {
      saved = await result.current.save();
    });

    expect(saved).toBeNull();
    expect(mocks.invoke.mock.calls.some((call) => call[0] === "save_file")).toBe(
      false,
    );
  });

  it("auto-runs exactly once after the debounce window", async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSandbox());

    act(() => {
      result.current.setCode("print('one')");
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(AUTO_RUN_DEBOUNCE_MS * 3);
    });

    const runs = mocks.invoke.mock.calls.filter(
      (call) => call[0] === "run_code",
    );
    expect(runs).toHaveLength(1);
    expect(runs[0][1].payload.code).toBe("print('one')");

    vi.useRealTimers();
  });

  it("does not auto-run when the toggle is off", async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useSandbox());

    act(() => {
      result.current.toggleAutoRun();
      result.current.setCode("print('two')");
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(AUTO_RUN_DEBOUNCE_MS * 3);
    });

    const runs = mocks.invoke.mock.calls.filter(
      (call) => call[0] === "run_code",
    );
    expect(runs).toHaveLength(0);

    vi.useRealTimers();
  });

  it("auto-runs the file that was opened", async () => {
    vi.useFakeTimers();
    mocks.openDialog.mockResolvedValue("/tmp/main.rs");
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "open_file") {
        return { path: "/tmp/main.rs", contents: "fn main() {}\n" };
      }
      return [];
    });

    const { result } = renderHook(() => useSandbox());

    await act(async () => {
      await result.current.open();
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(AUTO_RUN_DEBOUNCE_MS * 3);
    });

    const runs = mocks.invoke.mock.calls.filter(
      (call) => call[0] === "run_code",
    );
    // The buffer changed, so the auto-run kicks in — exactly like after a keystroke.
    expect(runs).toHaveLength(1);
    expect(runs[0][1].payload.code).toBe("fn main() {}\n");

    vi.useRealTimers();
  });

  it("manages zoom level between 70% and 200%", async () => {
    const { result } = await renderStore();
    expect(result.current.zoom).toBe(100);

    act(() => {
      result.current.zoomIn();
    });
    expect(result.current.zoom).toBe(110);
    expect(document.documentElement.style.zoom).toBe("110%");
    expect(document.documentElement.style.getPropertyValue("--ui-zoom")).toBe(
      "110%",
    );

    // Zoom in repeatedly to test upper bound of 200%
    for (let i = 0; i < 15; i++) {
      act(() => {
        result.current.zoomIn();
      });
    }
    expect(result.current.zoom).toBe(200);

    // Reset zoom
    act(() => {
      result.current.resetZoom();
    });
    expect(result.current.zoom).toBe(100);

    // Zoom out repeatedly to test lower bound of 70%
    for (let i = 0; i < 10; i++) {
      act(() => {
        result.current.zoomOut();
      });
    }
    expect(result.current.zoom).toBe(70);

    // Direct clamp via setZoom
    act(() => {
      result.current.setZoom(140);
    });
    expect(result.current.zoom).toBe(140);

    act(() => {
      result.current.setZoom(500);
    });
    expect(result.current.zoom).toBe(200);
  });
});
