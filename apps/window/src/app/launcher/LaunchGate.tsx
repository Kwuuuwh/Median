import { type Catalog, commands, send } from "@median/ipc";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { dataProgress } from "@/shared/lib/data-progress";
import { Button } from "@/shared/ui/button";
import { GateBar } from "./GateBar";

/** Game data install progress in the launcher, or why the install stopped and what to do. */
export function LaunchGate({ catalog }: { catalog: Catalog }) {
  const t = useT();
  const locale = useLocale();
  const shown = dataProgress(t, locale, catalog);
  const activity = catalog.activity;
  const share =
    activity.state === "fetching" && activity.total > 0 ? activity.done / activity.total : null;

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3">
      <div className="font-display font-semibold text-heading">{shown.title}</div>
      {shown.detail && <div className="text-muted tabular-nums">{shown.detail}</div>}
      {share !== null && <GateBar share={share} />}
      {shown.retry && (
        <Button onClick={() => send(commands.syncCatalog())}>{t("gate-offline-retry")}</Button>
      )}
    </div>
  );
}
