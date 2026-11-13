import { describe, expect, it, vi } from "vitest";

import { setVimRunHandler, triggerVimRun } from "../lib/vim";

describe("the VIM integration", () => {
  it("runs the program when the handler is triggered", () => {
    const handler = vi.fn();
    setVimRunHandler(handler);

    triggerVimRun();

    expect(handler).toHaveBeenCalledTimes(1);
    setVimRunHandler(null);
  });

  it("overrides the `write` ex-command so that `:w` runs instead of saving", async () => {
    vi.resetModules();
    const defineEx = vi.fn();

    vi.doMock("@replit/codemirror-vim", async () => {
      const actual = await vi.importActual<
        typeof import("@replit/codemirror-vim")
      >("@replit/codemirror-vim");
      return {
        ...actual,
        Vim: { ...actual.Vim, defineEx },
      };
    });

    await import("../lib/vim");

    expect(defineEx).toHaveBeenCalledWith(
      "write",
      undefined,
      expect.any(Function),
    );

    vi.doUnmock("@replit/codemirror-vim");
    vi.resetModules();
  });
});
