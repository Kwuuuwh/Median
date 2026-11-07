import { Bell, X } from "lucide-react";
import { useNodes } from "@/shared/data/catalog";
import { useWatches, useWatchToggle, watchKey } from "@/shared/data/watches";
import { useT } from "@/shared/i18n";
import { ageUnit } from "@/shared/lib/format";
import { nowSecs } from "@/shared/lib/use-now";
import { Button } from "@/shared/ui/button";
import { EmptyLine } from "@/shared/ui/empty-state";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";
import { clearFeed, dismissAlarm, markSeen, useFeed } from "../model/feed";
import { alarmPlace, watchLabel } from "../model/label";

/** Title bar bell with a popover of this session's alarms and the active watches. */
export function AlarmCenter() {
  const t = useT();
  const { rung, unseen } = useFeed();
  const watches = useWatches();
  const toggle = useWatchToggle();
  const nodes = useNodes([
    ...new Set(rung.flatMap((each) => (each.alarm.node ? [each.alarm.node] : []))),
  ]).data;

  const ago = (at: number) => {
    const { unit, count } = ageUnit(nowSecs() - at);
    return unit === "fresh" ? t("price-age-fresh") : t(`price-age-${unit}`, { count });
  };

  return (
    <Popover onOpenChange={(open) => open && markSeen()}>
      <PopoverTrigger asChild>
        <Button size="icon-sm" tone="ghost" aria-label={t("alarms-title")} className="relative">
          <Bell />
          {unseen > 0 && (
            <span className="absolute top-0.5 right-0.5 flex min-w-3.5 items-center justify-center rounded-full bg-accent px-1 font-semibold text-canvas text-caption leading-none">
              {unseen}
            </span>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-96 p-0">
        <section className="border-line border-b p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="font-medium text-muted text-small">{t("alarms-title")}</h3>
            {rung.length > 0 && (
              <button
                type="button"
                onClick={clearFeed}
                className="text-dim text-small hover:text-ink"
              >
                {t("alarms-clear")}
              </button>
            )}
          </div>
          {rung.length === 0 ? (
            <EmptyLine className="py-2">{t("alarms-none")}</EmptyLine>
          ) : (
            <ul className="max-h-72 divide-y divide-line overflow-y-auto">
              {rung.map((each) => (
                <li key={each.id} className="group grid grid-cols-[1fr_auto_auto] gap-x-2 py-2">
                  <span className="text-body text-ink">{watchLabel(t, each.alarm.watch)}</span>
                  <span className="text-dim text-small tabular-nums">{ago(each.at)}</span>
                  <button
                    type="button"
                    aria-label={t("alarms-dismiss")}
                    onClick={() => dismissAlarm(each.id)}
                    className="text-dim opacity-0 hover:text-ink group-hover:opacity-100"
                  >
                    <X className="size-3.5" />
                  </button>
                  {each.alarm.node && (
                    <span className="col-span-3 text-muted text-small">
                      {alarmPlace(nodes?.get(each.alarm.node)) ?? each.alarm.node}
                    </span>
                  )}
                </li>
              ))}
            </ul>
          )}
        </section>
        <section className="p-3">
          <h3 className="mb-2 font-medium text-muted text-small">{t("alarms-watches")}</h3>
          {watches.length === 0 ? (
            <div className="py-2 text-dim text-small">{t("alarms-no-watches")}</div>
          ) : (
            <ul className="divide-y divide-line">
              {watches.map((watch) => (
                <li key={watchKey(watch)} className="group flex h-9 items-center gap-2">
                  <span className="flex-1 text-body text-soft">{watchLabel(t, watch)}</span>
                  <Button
                    size="icon-sm"
                    tone="ghost"
                    aria-label={t("alarms-drop")}
                    onClick={() => toggle.mutate({ watch, on: false })}
                    className="opacity-0 group-hover:opacity-100"
                  >
                    <X />
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </PopoverContent>
    </Popover>
  );
}
