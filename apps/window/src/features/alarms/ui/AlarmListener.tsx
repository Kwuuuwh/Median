import { events } from "@median/ipc";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { fetchNodes } from "@/shared/data/catalog";
import { useT } from "@/shared/i18n";
import { notifySystem } from "@/shared/lib/notify";
import { nowSecs } from "@/shared/lib/use-now";
import { showNotice } from "@/shared/ui/toast";
import { recordAlarms } from "../model/feed";
import { alarmPlace, watchLabel } from "../model/label";

/**
 * Listens for triggered watches and adds each alarm to the feed, shows it as a toast, and as a
 * system notification while the window is out of focus; renders nothing.
 */
export function AlarmListener() {
  const t = useT();
  const client = useQueryClient();

  const label = useRef({ t });
  useEffect(() => {
    label.current = { t };
  });

  useEffect(() => {
    const stop = events.rang.listen(async ({ payload }) => {
      const { t } = label.current;
      recordAlarms(payload.alarms, nowSecs());
      const nodes = await fetchNodes(
        client,
        payload.alarms.flatMap((alarm) => (alarm.node ? [alarm.node] : [])),
      );
      for (const alarm of payload.alarms) {
        const title = watchLabel(t, alarm.watch);
        const detail = alarm.node ? alarmPlace(nodes.get(alarm.node)) : undefined;
        showNotice({ tone: "alarm", title, detail });
        void notifySystem(title, detail);
      }
    });
    return () => void stop.then((unlisten) => unlisten());
  }, [client]);

  return null;
}
