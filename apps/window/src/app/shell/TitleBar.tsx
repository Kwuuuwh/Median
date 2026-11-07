import type { AppState } from "@median/ipc";
import { AlarmCenter } from "@/features/alarms/ui/AlarmCenter";
import { MarketStatus } from "./MarketStatus";
import { Troubles } from "./Troubles";
import { WhisperListener } from "./Whisper";
import { WindowButtons } from "./WindowButtons";

/**
 * Custom title bar: app name, profile and drag area, problem badge, market account control, alarms
 * and window buttons.
 */
export function TitleBar({ state }: { state: AppState }) {
  return (
    <header className="flex h-10 shrink-0 items-center border-hairline border-b bg-surface pl-4">
      <div data-tauri-drag-region className="flex h-full flex-1 items-center gap-3">
        <span
          data-tauri-drag-region
          className="font-display font-semibold text-soft text-title tracking-tight"
        >
          Median
        </span>
        {state.account.profile && (
          <span data-tauri-drag-region className="text-dim text-small">
            {state.account.profile}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2 pr-2">
        <Troubles state={state} />
        <WhisperListener whispers={state.game.whispers} />
        <MarketStatus membership={state.membership} />
        <AlarmCenter />
      </div>
      <WindowButtons />
    </header>
  );
}
