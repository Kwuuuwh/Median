import { events } from "@median/ipc";
import { useEffect, useRef } from "react";
import { useAppState } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { notifySystem } from "@/shared/lib/notify";
import { showNotice } from "@/shared/ui/toast";

/**
 * Listens for riven auctions the agent picks and shows each new pick as a toast and a system
 * notification; renders nothing.
 */
export function PickListener() {
  const t = useT();
  const picks = useAppState()?.agent.picks;
  const known = useRef<Set<string> | null>(null);
  const label = useRef({ t });
  useEffect(() => {
    label.current = { t };
  });
  if (known.current === null && picks) known.current = new Set(picks.map((pick) => pick.id));

  useEffect(() => {
    const stop = events.agentChanged.listen(({ payload }) => {
      const { t } = label.current;
      const seen = known.current ?? new Set<string>();
      for (const pick of payload.agent.picks) {
        if (seen.has(pick.id)) continue;
        const title = t("pick-notice");
        const detail = t("pick-notice-detail", {
          name: pick.name,
          buyout: pick.buyout,
          worth: pick.worth,
        });
        showNotice({ tone: "alarm", title, detail });
        void notifySystem(title, detail);
      }
      known.current = new Set(payload.agent.picks.map((pick) => pick.id));
    });
    return () => void stop.then((unlisten) => unlisten());
  }, []);

  return null;
}
