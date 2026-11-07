import type { MasteryRank, MasteryShare } from "@median/ipc";
import { ChartPie } from "lucide-react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { formatCount } from "@/shared/lib/format";
import { Button } from "@/shared/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";
import { reached } from "../model/rank";
import { TestCountdown } from "./TestCountdown";

/** Mastery sources, in the order the game lists them. */
const SOURCES = [
  "items",
  "nodes",
  "steelPath",
  "junctions",
  "steelJunctions",
  "railjack",
  "drifter",
] as const;

/** `part` as a CSS percentage of `whole`, clamped to 0–100%; `0%` when `whole` is 0. */
function share(part: number, whole: number): string {
  return `${whole > 0 ? Math.max(0, Math.min(100, (part / whole) * 100)) : 0}%`;
}

/** The account's rank, progress to the next rank, and the progress `preview` more mastery adds. */
export function RankStrip({ standing, preview }: { standing: MasteryRank; preview: number }) {
  const t = useT();
  const locale = useLocale();
  const floor = standing.floor ?? 0;
  const next = standing.next ?? standing.total;
  const span = next - floor;
  const earned = standing.total - floor;
  const lands = reached(standing.total + preview);
  return (
    <div className="flex shrink-0 items-center gap-5 border-line border-b px-6 py-3">
      <div className="flex items-baseline gap-1.5">
        <span className="font-display font-semibold text-figure text-ink tabular-nums">
          {standing.rank ?? "—"}
        </span>
        <span className="text-muted text-small">{t("mastery-rank-label")}</span>
      </div>
      <div className="grid min-w-0 flex-1 gap-1.5">
        <div className="relative h-2 overflow-hidden rounded-full bg-sunken">
          <div
            className="absolute inset-y-0 left-0 rounded-full bg-accent"
            style={{ width: share(earned, span) }}
          />
          <div
            className="absolute inset-y-0 bg-accent/35"
            style={{ left: share(earned, span), width: share(standing.inHand, span) }}
          />
          {preview > 0 && (
            <div
              className="absolute inset-y-0 bg-live/70"
              style={{ left: share(earned, span), width: share(preview, span) }}
            />
          )}
        </div>
        <div className="flex flex-wrap items-baseline gap-x-3 text-small tabular-nums">
          <span className="text-soft">
            {t(standing.exact ? "mastery-progress" : "mastery-progress-floor", {
              total: formatCount(locale, standing.total),
              next: formatCount(locale, next),
            })}
          </span>
          {standing.remaining !== null && (
            <span className="text-muted">
              {t("mastery-remaining", { remaining: formatCount(locale, standing.remaining) })}
            </span>
          )}
          {standing.inHand > 0 && (
            <span className="text-muted">
              {t("mastery-in-hand", { points: formatCount(locale, standing.inHand) })}
            </span>
          )}
          {preview > 0 && (
            <span className="text-live">
              {lands > (standing.rank ?? 0)
                ? t("mastery-gain-rank", { points: formatCount(locale, preview), rank: lands })
                : t("mastery-gain", { points: formatCount(locale, preview) })}
            </span>
          )}
        </div>
      </div>
      {standing.test &&
        standing.test.state !== "unearned" &&
        (standing.test.state === "open" ? (
          <span className="rounded-full bg-live/15 px-2.5 py-1 text-live text-small">
            {t("mastery-test-open")}
          </span>
        ) : (
          <span className="rounded-full bg-sunken px-2.5 py-1 text-muted text-small">
            <TestCountdown until={standing.test.until} />
          </span>
        ))}
      {standing.undercounted && (
        <span className="rounded-full bg-danger/15 px-2.5 py-1 text-danger text-small">
          {t("mastery-undercounted")}
        </span>
      )}
      <Popover>
        <PopoverTrigger asChild>
          <Button tone="ghost" size="icon" aria-label={t("mastery-sources")}>
            <ChartPie />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="w-72">
          <h3 className="mb-2 font-medium text-body text-ink">{t("mastery-sources")}</h3>
          <ul className="grid gap-1.5">
            {SOURCES.map((source) => {
              const counted: MasteryShare = standing[source];
              return (
                <li key={source} className="flex items-baseline gap-3 text-small">
                  <span className="flex-1 text-soft">{t("mastery-source", { source })}</span>
                  <span className="text-muted tabular-nums">
                    {t("mastery-share", { done: counted.done, of: counted.of })}
                  </span>
                  <span className="w-20 text-right text-ink tabular-nums">
                    {formatCount(locale, counted.earned)}
                  </span>
                </li>
              );
            })}
          </ul>
          {standing.uncounted.length > 0 && (
            <p className="mt-2 text-danger text-small">
              {standing.uncounted.map((source) => t("mastery-uncounted", { source })).join(", ")}
            </p>
          )}
        </PopoverContent>
      </Popover>
    </div>
  );
}
