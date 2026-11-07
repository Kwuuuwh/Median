import type { commands, Result } from "@median/ipc";

type Commands = typeof commands;

/** Command name as a key of `commands`. */
export type Command = keyof Commands;

/** A command's return value with its `Result` wrapper, if any, removed. */
export type Answer<K extends Command> =
  Awaited<ReturnType<Commands[K]>> extends Result<infer T, unknown>
    ? T
    : Awaited<ReturnType<Commands[K]>>;

/** Optional mock per command, taking the command's own arguments; a throw is an error. */
export type Answers = {
  [K in Command]?: (...args: Parameters<Commands[K]>) => Answer<K> | Promise<Answer<K>>;
};
