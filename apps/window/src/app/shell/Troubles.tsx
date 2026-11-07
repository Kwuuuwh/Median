import { type AppState, commands, send } from "@median/ipc";
import { useQuery } from "@tanstack/react-query";
import { FolderOpen, TriangleAlert } from "lucide-react";
import { useLocale } from "@/shared/data/app-state";
import { keys } from "@/shared/data/keys";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { useNow } from "@/shared/lib/use-now";
import { Button } from "@/shared/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";
import { type Source, troublesOf } from "./health";

/** Short label of each source in the list; a task problem shows the task name instead. */
const SOURCE: Record<Source, string> = {
  gep: "GEP",
  dbwin: "DBWIN",
  ocr: "OCR",
  wfm: "WFM",
  task: "",
};

/**
 * Warning badge with the problem count and a popover listing each problem; renders nothing while
 * there are none. Red when any problem is severe, amber otherwise.
 */
export function Troubles({ state }: { state: AppState }) {
  const t = useT();
  const locale = useLocale();
  const troubles = troublesOf(state, useNow());
  if (troubles.length === 0) return null;
  const severe = troubles.some((each) => each.severe);
  const clock = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" });
  return (
    <Popover>
      <PopoverTrigger
        aria-label={t("troubles-title")}
        className={cn(
          "inline-flex h-7 items-center gap-1.5 rounded-control px-2 font-medium text-small tabular-nums transition-colors",
          severe
            ? "bg-danger/15 text-danger hover:bg-danger/25"
            : "bg-warn/15 text-warn hover:bg-warn/25",
        )}
      >
        <TriangleAlert className="size-3.5" />
        {troubles.length}
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80 p-0">
        <ul className="divide-y divide-hairline">
          {troubles.map((each) => (
            <li
              key={`${each.source}:${each.said}:${each.name ?? ""}`}
              className="flex gap-2.5 px-3 py-2.5"
            >
              <span
                className={cn(
                  "mt-1.5 size-1.5 shrink-0 rounded-full",
                  each.severe ? "bg-danger" : "bg-warn",
                )}
              />
              <div className="grid min-w-0 gap-0.5">
                <span className="text-body text-ink">
                  {each.source !== "task" && (
                    <span className="mr-1.5 font-medium text-soft">{SOURCE[each.source]}</span>
                  )}
                  {t(each.said, { name: each.name ?? "" })}
                </span>
                {each.since !== undefined && (
                  <span className="text-muted text-small tabular-nums">
                    {t("trouble-since", { time: clock.format(each.since * 1000) })}
                  </span>
                )}
                {each.reason && (
                  <span className="wrap-anywhere text-muted text-small">{each.reason}</span>
                )}
                {each.loadsCollector && <CollectorHelp />}
              </div>
            </li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  );
}

/** How to load the collector into Overwolf, with a button that opens its folder. */
function CollectorHelp() {
  const t = useT();
  const folder = useQuery({
    queryKey: keys.collectorFolder,
    queryFn: () => commands.getCollectorFolder(),
  }).data;
  if (!folder) return null;
  return (
    <div className="mt-1 grid gap-1.5">
      <span className="text-muted text-small">{t("collector-load")}</span>
      <Button
        size="sm"
        className="justify-self-start"
        onClick={() => send(commands.showCollectorFolder())}
      >
        <FolderOpen />
        {t("collector-folder-open")}
      </Button>
    </div>
  );
}
