import { mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Answers } from "./answers";

/** The camel-case `commands` key of a snake-case Rust command name. */
function commandKey(rust: string): string {
  return rust.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase());
}

/** Routes every IPC command to its mock implementation; a command without one throws. */
export function installStand(answers: Answers): void {
  mockWindows("main");
  mockConvertFileSrc("windows");
  mockIPC(
    (rust, payload) => {
      const answer = (answers as Record<string, (...args: unknown[]) => unknown>)[commandKey(rust)];
      if (!answer) throw `${rust} has no answer on the stand`;
      return answer(...Object.values(payload ?? {}));
    },
    { shouldMockEvents: true },
  );
}
