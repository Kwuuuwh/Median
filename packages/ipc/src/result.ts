export type Result<T, E> = { status: "ok"; data: T } | { status: "error"; error: E };

/// A command that came back rejected.
export class CommandError extends Error {
  constructor(readonly reason: unknown) {
    super(String(reason));
    this.name = "CommandError";
  }
}

/// The data a command returned; a rejection throws.
export async function unwrap<T, E>(call: Promise<Result<T, E>>): Promise<T> {
  const result = await call;
  if (result.status === "error") throw new CommandError(result.error);
  return result.data;
}

let report: (error: CommandError) => void = (error) => console.error(error);

/// Redirects rejections of fired commands; the default writes them to the console.
export function onCommandError(sink: (error: CommandError) => void): void {
  report = sink;
}

/// Fires a command whose answer arrives as state; a rejection is reported.
export function send<E>(call: Promise<Result<null, E>>): void {
  void unwrap(call).catch((error: unknown) => {
    report(error instanceof CommandError ? error : new CommandError(error));
  });
}
