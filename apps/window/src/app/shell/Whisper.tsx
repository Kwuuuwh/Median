import type { Whisper as Heard } from "@median/ipc";
import { useEffect, useRef } from "react";
import { useT } from "@/shared/i18n";
import { notifySystem } from "@/shared/lib/notify";
import { nowSecs } from "@/shared/lib/use-now";

/**
 * Shows a system notification for the latest in-game whisper when it is newer than the last one
 * handled, while the window is out of focus; renders nothing.
 */
export function WhisperListener({ whispers }: { whispers: Heard[] }) {
  const t = useT();
  const notified = useRef(nowSecs());
  const latest = whispers.reduce<Heard | null>(
    (last, each) => (!last || each.at > last.at ? each : last),
    null,
  );

  useEffect(() => {
    if (!latest || latest.at < notified.current) return;
    notified.current = latest.at + 1;
    void notifySystem(t("whisper-writes", { player: latest.player }));
  }, [latest, t]);

  return null;
}
