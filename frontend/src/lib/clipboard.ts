/**
 * Clipboard access with a portable, multi-route implementation.
 *
 * The user requirement is a *native* feel: `Ctrl+C` / `Ctrl+V` on Windows and Linux and
 * `Cmd+C` / `Cmd+V` on macOS. The routes are tried in this order:
 *
 * 1. the asynchronous DOM clipboard API (works in Chromium and modern WebKitGTK builds),
 * 2. the legacy `document.execCommand("copy")` route (works in every webview, synchronously
 *    and therefore inside the user-gesture window),
 * 3. the native Tauri clipboard (always available inside the application, but asynchronous).
 *
 * Reading is slightly different: the DOM route is preferred but raced against a short timeout,
 * because a webview that implements `navigator.clipboard.readText` but cannot grant the
 * permission would otherwise hang the paste forever. The native route is the fallback.
 */

import { clipboardReadText, clipboardWriteText } from "./api";

/** Whether the app runs on macOS — the platform where the modifier is `Cmd` instead of `Ctrl`. */
export function isMac(): boolean {
  if (typeof navigator === "undefined") return false;
  const nav = navigator as Navigator & {
    userAgentData?: { platform?: string };
  };
  const platform = nav.userAgentData?.platform ?? navigator.platform ?? "";
  const agent = navigator.userAgent ?? "";
  return /mac|iphone|ipad|ipod/i.test(platform) || /mac os x/i.test(agent);
}

/**
 * The label of the platform's primary modifier — `⌘` on macOS, `Ctrl` everywhere else.
 * Used in the status bar hints so the user always sees the right key.
 */
export function modifierLabel(): string {
  return isMac() ? "⌘" : "Ctrl";
}

/** Resolves after `ms` milliseconds. */
function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Runs `promise` but gives up after `ms` milliseconds. */
async function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<never>((_resolve, reject) => {
        timer = setTimeout(() => reject(new Error("timeout")), ms);
      }),
    ]);
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}

/** Copies text using the DOM clipboard API. */
async function domWrite(text: string): Promise<boolean> {
  try {
    if (typeof navigator === "undefined" || !navigator.clipboard?.writeText) return false;
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** Copies text using the legacy `execCommand` route. */
function execCommandWrite(text: string): boolean {
  if (typeof document === "undefined") return false;
  try {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.top = "-1000px";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    const ok = document.execCommand("copy");
    document.body.removeChild(area);
    return ok;
  } catch {
    return false;
  }
}

/**
 * Writes text to the system clipboard.
 *
 * @returns `true` when one of the routes succeeded.
 */
export async function writeClipboard(text: string): Promise<boolean> {
  if (await domWrite(text)) return true;
  if (execCommandWrite(text)) return true;
  try {
    await clipboardWriteText(text);
    return true;
  } catch {
    return false;
  }
}

/**
 * Reads text from the system clipboard.
 *
 * @returns the clipboard contents, or an empty string when nothing could be read.
 */
export async function readClipboard(): Promise<string> {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard?.readText) {
      const text = await withTimeout(navigator.clipboard.readText(), 400);
      if (typeof text === "string") return text;
    }
  } catch {
    // Fall through to the native route.
  }
  try {
    const text = await clipboardReadText();
    return typeof text === "string" ? text : "";
  } catch {
    return "";
  }
}

/** Small helper used by the tests to wait for the pending timers to settle. */
export async function settle(ms = 0): Promise<void> {
  await delay(ms);
}
