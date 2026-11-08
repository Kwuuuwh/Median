import { commands } from "@median/ipc";
import { useCallback, useEffect, useState } from "react";
import { WindowButtons } from "@/app/shell/WindowButtons";
import { useAppState, useBuild, useLocale, useStartFailure } from "@/shared/data/app-state";
import { accentPaint } from "@/shared/lib/accent";
import { revisionText } from "@/shared/lib/revision";
import { LaunchGate } from "./LaunchGate";
import { StartFailed } from "./StartFailed";
import { UpdateGate } from "./UpdateGate";

/**
 * The launcher window: the app update when the build takes updates, then the game data; after a
 * failed start, why it failed and the update that may fix it.
 */
export function Launcher() {
  const state = useAppState();
  const build = useBuild();
  const failure = useStartFailure();
  const locale = useLocale();
  const [settled, setSettled] = useState(false);
  const settle = useCallback(() => {
    setSettled(true);
    void commands.updateSettled();
  }, []);

  useEffect(() => {
    if (!state) return;
    document.documentElement.style.setProperty("--paint-accent", accentPaint(state.accent));
  }, [state]);

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-10 shrink-0 items-center border-hairline border-b bg-surface pl-4">
        <span
          data-tauri-drag-region
          className="flex h-full flex-1 items-center font-display font-semibold text-soft text-title tracking-tight"
        >
          Median
        </span>
        <WindowButtons maximize={false} />
      </header>
      <main data-tauri-drag-region className="flex min-h-0 flex-1 px-6">
        {failure ? (
          build && <StartFailed failure={failure} build={build} />
        ) : build?.updates && !settled ? (
          <UpdateGate onSettled={settle} />
        ) : (
          build && state && <LaunchGate catalog={state.catalog} />
        )}
      </main>
      {build && (
        <footer className="shrink-0 pb-3 text-center text-caption text-dim tabular-nums">
          {revisionText(build, locale)}
        </footer>
      )}
    </div>
  );
}
