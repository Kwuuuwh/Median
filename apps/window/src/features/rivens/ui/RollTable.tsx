import type { AppraisedStat, RerollOdds } from "@median/ipc";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { CurrencyIcon } from "@/shared/ui/currency-icon";
import { bestLock, chanceText, kuvaText } from "../model/odds";
import { statText } from "../model/stat";

const ROW = "grid grid-cols-[minmax(0,1fr)_3.5rem_4.5rem_5.5rem] items-baseline gap-2";

/**
 * Reroll table, one row per lock option: chance of a higher-priced roll, average price of such
 * rolls, and expected kuva; the option with the lowest expected kuva is marked.
 */
export function RollTable({
  odds,
  stats,
}: {
  odds: readonly RerollOdds[];
  stats: readonly AppraisedStat[];
}) {
  const t = useT();
  const locale = useLocale();
  const best = bestLock(odds);
  return (
    <div className="grid gap-0.5 text-small">
      <span className={cn(ROW, "pb-1 text-caption text-dim")}>
        <span>{t("riven-lock")}</span>
        <span className="text-right">{t("riven-chance")}</span>
        <span className="inline-flex items-center justify-end gap-1">
          <CurrencyIcon currency="platinum" />
          {t("riven-better")}
        </span>
        <span className="text-right">{t("riven-kuva-head")}</span>
      </span>
      {odds.map((each) => {
        const stat = stats.find((one) => one.tag === each.locked);
        return (
          <span
            key={each.locked ?? "none"}
            className={cn(
              ROW,
              "rounded-control px-1 py-0.5 tabular-nums",
              each === best && "bg-accent/8",
            )}
          >
            <span className={cn("truncate", stat?.isCurse ? "text-danger" : "text-soft")}>
              {each.locked === null
                ? t("riven-lock-none")
                : stat
                  ? statText(stat, locale)
                  : each.locked}
            </span>
            <span className="text-right text-soft">{chanceText(locale, each.chance ?? 0)}</span>
            <span className="text-right text-soft">
              {each.betterMean === null ? "—" : formatCount(locale, each.betterMean ?? 0)}
            </span>
            <span className="text-right text-muted">
              {kuvaText(locale, each.expectedKuva ?? null)}
            </span>
          </span>
        );
      })}
    </div>
  );
}
