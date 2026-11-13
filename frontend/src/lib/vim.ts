/**
 * The VIM integration.
 *
 * Decision D1: in VIM mode `:w` **runs** the program — it never writes a file. The VIM core
 * resolves `:w` to its `write` ex-command (see `defaultExCommandMap` in
 * `@replit/codemirror-vim-core`), so the `write` command is overridden here.
 */
import { Vim } from "@replit/codemirror-vim";

type RunHandler = () => void;

let runHandler: RunHandler | null = null;

/** Registers the callback that `:w` triggers. */
export function setVimRunHandler(handler: RunHandler | null): void {
  runHandler = handler;
}

/** Runs the program — the action behind `:w`. */
export function triggerVimRun(): void {
  runHandler?.();
}

/** Overrides the VIM `write` ex-command so that `:w` runs the program. */
Vim.defineEx("write", undefined, () => {
  triggerVimRun();
});
