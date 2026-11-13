import { describe, expect, it, vi, afterEach } from "vitest";
import {
  clampZoom,
  registerZoomHandlers,
  triggerResetZoom,
  triggerZoomIn,
  triggerZoomOut,
  MAX_ZOOM,
  MIN_ZOOM,
} from "../lib/zoom";

describe("zoom management", () => {
  afterEach(() => {
    registerZoomHandlers(null);
  });

  it("clamps zoom levels to [70, 200]", () => {
    expect(clampZoom(50)).toBe(MIN_ZOOM);
    expect(clampZoom(70)).toBe(70);
    expect(clampZoom(100)).toBe(100);
    expect(clampZoom(150)).toBe(150);
    expect(clampZoom(200)).toBe(200);
    expect(clampZoom(250)).toBe(MAX_ZOOM);
  });

  it("returns false when no zoom handlers are registered", () => {
    registerZoomHandlers(null);
    expect(triggerZoomIn()).toBe(false);
    expect(triggerZoomOut()).toBe(false);
    expect(triggerResetZoom()).toBe(false);
  });

  it("triggers registered zoom handlers", () => {
    const handlers = {
      zoomIn: vi.fn(),
      zoomOut: vi.fn(),
      resetZoom: vi.fn(),
    };

    registerZoomHandlers(handlers);

    expect(triggerZoomIn()).toBe(true);
    expect(handlers.zoomIn).toHaveBeenCalledTimes(1);

    expect(triggerZoomOut()).toBe(true);
    expect(handlers.zoomOut).toHaveBeenCalledTimes(1);

    expect(triggerResetZoom()).toBe(true);
    expect(handlers.resetZoom).toHaveBeenCalledTimes(1);
  });
});
