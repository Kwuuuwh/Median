import { ChartNoAxesColumn } from "lucide-react";
import { useState } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useCurve, useQuotes } from "@/shared/data/market";
import { useT } from "@/shared/i18n";
import { formatCount, formatTenths } from "@/shared/lib/format";
import { nowSecs } from "@/shared/lib/use-now";
import { EmptyLine } from "@/shared/ui/empty-state";
import { Money } from "@/shared/ui/money";
import { Segmented } from "@/shared/ui/segmented";
import { dailyTrades, daysOf, layersOf } from "../../model/chart";
import { RANKS, type RankPick } from "../../model/orders";
import { PriceChart } from "../PriceChart";

type Period = "7" | "30" | "90";

/** An item's market strip: current price, trades per day, chart period and the price chart. */
export function ItemMarket({
  item,
  rank,
  onRank,
}: {
  item: string;
  rank: RankPick;
  onRank: (rank: RankPick) => void;
}) {
  const t = useT();
  const locale = useLocale();
  const [period, setPeriod] = useState<Period>("30");
  const curve = useCurve(item, "", Number(period), rank === "max").data;
  const quote = useQuotes([item]).data?.get(item) ?? null;
  const days = curve ? daysOf(curve, Number(period), nowSecs()) : [];
  const prices = days.flatMap((day) => (day.platinum === null ? [] : [day.platinum]));
  const stands = days.some((day) => day.selling !== null || day.buying !== null);
  const daily = curve ? dailyTrades(curve, nowSecs()) : null;
  const maxRank = curve?.maxRank ?? null;

  const periods = (
    <Segmented<Period>
      label={t("item-period")}
      className="h-7 justify-self-start"
      value={period}
      onChange={setPeriod}
      segments={(["7", "30", "90"] as const).map((each) => ({
        value: each,
        label: t("item-days", { days: Number(each) }),
      }))}
    />
  );
  const ranks = maxRank !== null && (
    <Segmented<RankPick>
      label={t("sell-rank")}
      className="h-7 justify-self-start"
      value={rank}
      onChange={onRank}
      segments={RANKS.map((each) => ({
        value: each,
        label: t("orders-rank-pick", { rank: each, max: maxRank }),
      }))}
    />
  );
  if (quote === null && prices.length === 0 && !stands) {
    return (
      <section className="flex items-center gap-4 border-hairline border-b px-4 py-3">
        <EmptyLine icon={ChartNoAxesColumn}>{t("item-untraded")}</EmptyLine>
        {ranks && <span className="ml-auto">{ranks}</span>}
      </section>
    );
  }
  if (prices.length === 0 && !stands) {
    return (
      <section className="flex items-center gap-4 border-hairline border-b px-4 py-3">
        {quote !== null && (
          <Money currency="platinum" amount={quote} className="font-semibold text-ink text-title" />
        )}
        <EmptyLine icon={ChartNoAxesColumn}>{t("item-no-trades")}</EmptyLine>
        <span className="ml-auto flex gap-2">
          {ranks}
          {periods}
        </span>
      </section>
    );
  }

  const stat = (label: string, value: string) => (
    <div className="flex items-baseline justify-between gap-3">
      <span className="text-muted text-small">{label}</span>
      <span className="font-medium text-body text-ink tabular-nums">{value}</span>
    </div>
  );
  return (
    <section className="flex gap-6 border-hairline border-b px-4 py-3">
      <div className="grid w-44 shrink-0 content-start gap-1.5">
        <Money currency="platinum" amount={quote} className="font-semibold text-figure text-ink" />
        {stat(t("item-daily"), daily === null ? "—" : formatTenths(locale, Math.round(daily * 10)))}
        {prices.length > 0 &&
          stat(
            t("item-range"),
            `${formatCount(locale, Math.min(...prices))}–${formatCount(locale, Math.max(...prices))}`,
          )}
        <div className="mt-1 grid gap-1.5">
          {ranks}
          {periods}
        </div>
      </div>
      <div className="min-w-0 flex-1">
        <PriceChart days={days} layers={layersOf(days)} height={88} />
      </div>
    </section>
  );
}
