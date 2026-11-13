/**
 * UI and Editor Zoom Management (G10 / D22).
 *
 * Supports scaling the entire UI and editor between 70% and 200% in 10% steps.
 * Provides keyboard shortcut integration for CodeMirror and global window listeners.
 */

export const MIN_ZOOM = 70;
export const MAX_ZOOM = 200;
export const DEFAULT_ZOOM = 100;
export const ZOOM_STEP = 10;

export interface ZoomHandlers {
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
}

let currentHandlers: ZoomHandlers | null = null;

/** Registers the active zoom handlers (typically connected to the useSandbox store). */
export function registerZoomHandlers(handlers: ZoomHandlers | null): void {
  currentHandlers = handlers;
}

/** Triggers a zoom in action. Returns true if handled. */
export function triggerZoomIn(): boolean {
  if (currentHandlers) {
    currentHandlers.zoomIn();
    return true;
  }
  return false;
}

/** Triggers a zoom out action. Returns true if handled. */
export function triggerZoomOut(): boolean {
  if (currentHandlers) {
    currentHandlers.zoomOut();
    return true;
  }
  return false;
}

/** Triggers a zoom reset action. Returns true if handled. */
export function triggerResetZoom(): boolean {
  if (currentHandlers) {
    currentHandlers.resetZoom();
    return true;
  }
  return false;
}

/** Clamps a zoom level between MIN_ZOOM and MAX_ZOOM. */
export function clampZoom(value: number): number {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, value));
}
