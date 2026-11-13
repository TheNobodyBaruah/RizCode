import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";

const mocks = vi.hoisted(() => ({
  clipboardWriteText: vi.fn(async () => undefined),
  clipboardReadText: vi.fn(async () => "from-tauri"),
}));

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    clipboardWriteText: mocks.clipboardWriteText,
    clipboardReadText: mocks.clipboardReadText,
  };
});

import { readClipboard, writeClipboard } from "../lib/clipboard";

/** Installs a `navigator.clipboard` stub. */
function stubClipboard(clipboard: unknown) {
  Object.defineProperty(navigator, "clipboard", {
    value: clipboard,
    configurable: true,
    writable: true,
  });
}

describe("clipboard", () => {
  beforeEach(() => {
    mocks.clipboardWriteText.mockClear();
    mocks.clipboardReadText.mockClear();
    stubClipboard(undefined);
  });

  afterEach(() => {
    stubClipboard(undefined);
  });

  it("writes through the DOM clipboard API when it is available", async () => {
    const writeText = vi.fn(async () => undefined);
    stubClipboard({ writeText });

    await expect(writeClipboard("hello")).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith("hello");
    expect(mocks.clipboardWriteText).not.toHaveBeenCalled();
  });

  it("falls back to the native command when the DOM API rejects", async () => {
    stubClipboard({
      writeText: vi.fn(async () => {
        throw new Error("denied");
      }),
    });

    // `document.execCommand` is not implemented by jsdom, so the native route is used.
    await expect(writeClipboard("hello")).resolves.toBe(true);
    expect(mocks.clipboardWriteText).toHaveBeenCalledWith("hello");
  });

  it("falls back to the native command when no DOM clipboard exists", async () => {
    await expect(writeClipboard("hello")).resolves.toBe(true);
    expect(mocks.clipboardWriteText).toHaveBeenCalledWith("hello");
  });

  it("reads through the DOM clipboard API when it is available", async () => {
    const readText = vi.fn(async () => "from-dom");
    stubClipboard({ readText });

    await expect(readClipboard()).resolves.toBe("from-dom");
    expect(readText).toHaveBeenCalled();
    expect(mocks.clipboardReadText).not.toHaveBeenCalled();
  });

  it("falls back to the native command when the DOM API is missing", async () => {
    await expect(readClipboard()).resolves.toBe("from-tauri");
    expect(mocks.clipboardReadText).toHaveBeenCalled();
  });

  it("falls back to the native command when the DOM API rejects", async () => {
    stubClipboard({
      readText: vi.fn(async () => {
        throw new Error("denied");
      }),
    });

    await expect(readClipboard()).resolves.toBe("from-tauri");
    expect(mocks.clipboardReadText).toHaveBeenCalled();
  });

  it("returns an empty string when nothing can be read", async () => {
    mocks.clipboardReadText.mockRejectedValueOnce(new Error("no clipboard"));
    await expect(readClipboard()).resolves.toBe("");
  });
});
