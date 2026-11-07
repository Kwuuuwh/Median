import type { Dealt, Position } from "@median/ipc";
import type { ReactNode } from "react";
import { openItem } from "@/features/items/model/open";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatDay } from "@/shared/lib/format";
import { useNow } from "@/shared/lib/use-now";
import { EmptyState } from "@/shared/ui/empty-state";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Money } from "@/shared/ui/money";
import { TileGrid } from "@/shared/ui/tile-grid";
import { Tooltip } from "@/shared/ui/tooltip";
import {
  byAgent,
  byGroup,
  daily,
  earners,
  type Grouped,
  markup,
  summarize,
  unsold,
} from "../model/stats";

/** Days the chart covers when the period is all time. */
const CHART_DAYS = 30;

/** Number of items in the top earners list. */
const EARNERS = 8;

/** A headline value tile: label, value, and an optional hint line under it. */
export function Figure({
  label,
  children,
  hint,
}: {
  label: string;
  children: ReactNode;
  hint?: ReactNode;
}) {
  return (
    <div className="grid gap-1 rounded-panel border border-line bg-surface px-4 py-3">
      <span className="text-muted text-small">{label}</span>
      <span className="font-display font-semibold text-figure text-ink tabular-nums">
        {children}
      </span>
      {hint && <span className="text-dim text-small tabular-nums">{hint}</span>}
    </div>
  );
}

/** Daily bar chart: sales above the midline and buys below it, with per-day totals on hover. */
function DayBars({ deals, days }: { deals: readonly Dealt[]; days: number }) {
  const t = useT();
  const locale = useLocale();
  const now = useNow();
  const series = daily(deals, days, now);
  const most = Math.max(1, ...series.flatMap((day) => [day.taken, day.spent]));
  return (
    <section className="rounded-panel border border-line bg-surface p-4">
      <header className="mb-3 flex items-center gap-4 text-small">
        <h3 className="text-muted">{t("agent-by-day")}</h3>
        <span className="ml-auto flex items-center gap-1.5 text-soft">
          <span className="size-2 rounded-sm bg-live/80" />
          {t("agent-sold")}
        </span>
        <span className="flex items-center gap-1.5 text-soft">
          <span className="size-2 rounded-sm bg-accent/70" />
          {t("agent-bought")}
        </span>
      </header>
      <div className="flex h-40 items-stretch gap-0.5">
        {series.map((each) => (
          <Tooltip
            key={each.starts}
            hint={
              <span className="grid gap-0.5 tabular-nums">
                <span className="text-muted">{formatDay(locale, each.starts)}</span>
                <span className="flex justify-between gap-4">
                  {t("agent-sold")}
                  <Money currency="platinum" amount={each.taken} />
                </span>
                <span className="flex justify-between gap-4">
                  {t("agent-bought")}
                  <Money currency="platinum" amount={each.spent} />
                </span>
                {each.margin !== 0 && (
                  <span className="flex justify-between gap-4">
                    {t("agent-profit")}
                    <Money currency="platinum" amount={each.margin} />
                  </span>
                )}
              </span>
            }
          >
            <span className="relative flex-1 rounded-sm hover:bg-raised">
              <span className="absolute inset-x-0 top-1/2 h-px bg-hairline" />
              {each.taken > 0 && (
                <span
                  className="absolute inset-x-0.5 bottom-1/2 rounded-t bg-live/80"
                  style={{ height: `${(each.taken / most) * 50}%` }}
                />
              )}
              {each.spent > 0 && (
                <span
                  className="absolute inset-x-0.5 top-1/2 rounded-b bg-accent/70"
                  style={{ height: `${(each.spent / most) * 50}%` }}
                />
              )}
            </span>
          </Tooltip>
        ))}
      </div>
      <div className="mt-1.5 flex justify-between text-caption text-dim tabular-nums">
        <span>{formatDay(locale, series[0].starts)}</span>
        <span>{formatDay(locale, series[series.length - 1].starts)}</span>
      </div>
    </section>
  );
}

/** One item group: sale margin, platinum from sales and spent on buys; dimmed without exchanges. */
function GroupTile({ grouped }: { grouped: Grouped }) {
  const t = useT();
  return (
    <article
      className={cn(
        "grid gap-1.5 rounded-panel border border-line bg-surface px-3.5 py-3",
        grouped.sales + grouped.buys === 0 && "opacity-50",
      )}
    >
      <span className="text-muted text-small">
        {t("figures-group", { group: grouped.group || "other" })}
      </span>
      <Money
        currency="platinum"
        amount={grouped.margin}
        className={cn(
          "font-semibold text-title tabular-nums",
          grouped.margin > 0 && "text-live",
          grouped.margin < 0 && "text-danger",
        )}
      />
      <span className="flex items-center gap-3 text-small tabular-nums">
        <span className="flex items-center gap-1 text-soft">
          <span className="size-1.5 rounded-full bg-live/80" />
          <Money currency="platinum" amount={grouped.taken} />
        </span>
        <span className="flex items-center gap-1 text-soft">
          <span className="size-1.5 rounded-full bg-accent/70" />
          <Money currency="platinum" amount={grouped.spent} />
        </span>
      </span>
    </article>
  );
}

/** A titled grid of headline value tiles. */
function Figures({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="mb-2 text-muted text-small">{title}</h3>
      <TileGrid least={176}>{children}</TileGrid>
    </section>
  );
}

/** Platinum amount coloured by sign: green above zero, red below. */
export function Signed({ amount }: { amount: number }) {
  return (
    <Money
      currency="platinum"
      amount={amount}
      className={cn(amount > 0 && "text-live", amount < 0 && "text-danger")}
    />
  );
}

/**
 * Agent statistics: current committed and unsold platinum, then the period's totals, daily chart,
 * totals per item group, and top earners; an empty state when the period has no agent exchanges.
 */
export function StatsView({
  deals,
  positions,
  bids,
  tied,
  purse,
  days,
  period,
}: {
  deals: readonly Dealt[];
  positions: readonly Position[];
  /** Number of open agent buy orders; `tied` is the platinum they commit. */
  bids: number;
  tied: number;
  /** Platinum the account holds; null until read. */
  purse: number | null;
  days: number;
  /** The control that selects `days`, rendered next to the period heading. */
  period: ReactNode;
}) {
  const t = useT();
  const shown = byAgent(deals);
  const sum = summarize(shown);
  const best = earners(shown, EARNERS);
  const groups = byGroup(shown);
  const held = unsold(positions);
  const marked = markup(shown);
  return (
    <div className="grid gap-5">
      <Figures title={t("agent-now")}>
        <Figure label={t("agent-in-bids")} hint={t("agent-bid-count", { count: bids })}>
          <Money currency="platinum" amount={tied} />
        </Figure>
        <Figure label={t("agent-unsold")} hint={t("agent-copies", { count: held.copies })}>
          <Money currency="platinum" amount={held.cost} />
        </Figure>
        <Figure
          label={t("agent-unsold-worth")}
          hint={held.copies > 0 && <Signed amount={held.worth - held.cost} />}
        >
          <Money currency="platinum" amount={held.worth} />
        </Figure>
        {purse !== null && (
          <Figure label={t("agent-free")}>
            <Money currency="platinum" amount={Math.max(0, purse - tied)} />
          </Figure>
        )}
      </Figures>
      <div className="flex flex-wrap items-center gap-3">
        <h3 className="text-muted text-small">{t("agent-period")}</h3>
        {period}
      </div>
      {shown.length === 0 ? (
        <EmptyState title={t("agent-no-stats")} />
      ) : (
        <>
          <TileGrid least={176}>
            <Figure label={t("agent-profit")} hint={t("agent-margined", { count: sum.margined })}>
              <Signed amount={sum.margin} />
            </Figure>
            <Figure label={t("agent-taken")} hint={t("agent-sales", { count: sum.sales })}>
              <Money currency="platinum" amount={sum.taken} />
            </Figure>
            <Figure label={t("agent-spent")} hint={t("agent-buys", { count: sum.buys })}>
              <Money currency="platinum" amount={sum.spent} />
            </Figure>
            {marked !== null && (
              <Figure label={t("agent-markup")}>
                <span className={cn(marked > 0 && "text-live", marked < 0 && "text-danger")}>
                  {marked}%
                </span>
              </Figure>
            )}
            {sum.margined > 0 && (
              <Figure label={t("agent-per-flip")}>
                <Signed amount={Math.round(sum.margin / sum.margined)} />
              </Figure>
            )}
          </TileGrid>
          <DayBars deals={shown} days={days > 0 ? days : CHART_DAYS} />
          <section>
            <h3 className="mb-2 text-muted text-small">{t("agent-by-group")}</h3>
            <TileGrid least={192}>
              {groups.map((each) => (
                <GroupTile key={each.group} grouped={each} />
              ))}
            </TileGrid>
          </section>
          {best.length > 0 && (
            <section>
              <h3 className="mb-2 text-muted text-small">{t("agent-earners")}</h3>
              <TileGrid least={192}>
                {best.map((earner) => (
                  <button
                    type="button"
                    key={earner.item || earner.name}
                    disabled={!earner.item}
                    onClick={() => openItem(earner.item)}
                    className="flex items-center gap-2.5 rounded-panel border border-line bg-surface p-2.5 text-left transition-colors hover:border-dim"
                  >
                    <ItemPicture
                      image={earner.image}
                      className="size-10 shrink-0 rounded-control bg-sunken p-0.5"
                    />
                    <span className="grid min-w-0">
                      <span className="truncate text-body text-ink">{earner.name}</span>
                      <span className="flex items-center gap-2 text-small tabular-nums">
                        <Money
                          currency="platinum"
                          amount={earner.margin}
                          className={earner.margin >= 0 ? "text-live" : "text-danger"}
                        />
                        <span className="text-muted">
                          {t("agent-sales", { count: earner.sales })}
                        </span>
                      </span>
                    </span>
                  </button>
                ))}
              </TileGrid>
            </section>
          )}
        </>
      )}
    </div>
  );
}
