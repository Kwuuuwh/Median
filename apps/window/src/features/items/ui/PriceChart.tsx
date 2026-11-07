import { useId, useState } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount, formatDay } from "@/shared/lib/format";
import { useWidth } from "@/shared/lib/use-width";
import { Money } from "@/shared/ui/money";
import { type Day, type Layer, lastPriced, nearestDay, priceOn, priceTicks } from "../model/chart";

/** Width in pixels reserved on the right for the price axis. */
const AXIS = 28;
/** Height in pixels of the trade volume strip under the line. */
const TRADES = 18;
/** Gap in pixels between the line and the trade volume strip. */
const GAP = 6;
/** Space in pixels above the line for the top price label. */
const HEAD = 7;

/** Colour of each series. */
const LAYER_TONE: Record<Layer, string> = {
  trades: "text-plat",
  selling: "text-warn",
  buying: "text-live",
};

/** Message id of each series' name. */
const LAYER_NAME = {
  trades: "chart-closed",
  selling: "orders-selling",
  buying: "orders-buying",
} as const;

/** Daily price chart: best sell and buy orders or closed-trade price, over volume bars. */
export function PriceChart({
  days,
  layers = ["trades"],
  mark,
  height = 88,
  className,
}: {
  days: readonly Day[];
  layers?: readonly Layer[];
  /** A time to mark on the line, such as a vendor's arrival. */
  mark?: number;
  /** Height in pixels of the price line area, without the trade volume strip. */
  height?: number;
  className?: string;
}) {
  const t = useT();
  const locale = useLocale();
  const gradient = useId();
  const [box, setBox] = useState<HTMLDivElement | null>(null);
  const [pointed, setPointed] = useState<number | null>(null);
  const width = useWidth(box);
  const last = lastPriced(days, layers);
  if (last < 0) return null;

  const showsTrades = layers.includes("trades");
  const traded = days.some((each) => each.trades > 0);
  const prices = days.flatMap((day) =>
    layers.flatMap((layer) => {
      const price = priceOn(day, layer);
      return price === null ? [] : [price];
    }),
  );
  const slack = (Math.max(...prices) - Math.min(...prices)) * 0.15 || 1;
  const low = Math.max(0, Math.min(...prices) - slack);
  const high = Math.max(...prices) + slack;
  const ticks = priceTicks(low, high);
  const plot = Math.max(0, width - AXIS);
  const most = Math.max(1, ...days.map((day) => day.trades));
  const first = days[0].at;
  const span = days[days.length - 1].at - first || 1;
  const x = (index: number) => (days.length === 1 ? plot / 2 : (index / (days.length - 1)) * plot);
  const y = (price: number) => height - ((price - low) / (high - low || 1)) * (height - HEAD);
  const pathOf = (layer: Layer) => {
    const points = days.flatMap((day, index) => {
      const price = priceOn(day, layer);
      return price === null ? [] : [`${x(index)},${y(price)}`];
    });
    return points.length > 0 ? `M${points.join("L")}` : null;
  };
  const tradeLine = pathOf("trades");
  const firstTraded = days.findIndex((day) => day.platinum !== null);
  const lastTraded = lastPriced(days, ["trades"]);
  const area = tradeLine && `${tradeLine}L${x(lastTraded)},${height}L${x(firstTraded)},${height}Z`;
  const bar = Math.max(1, plot / days.length - 1.5);
  const shown = pointed ?? last;
  const day = days[shown];

  return (
    <div className={cn("grid gap-1", className)}>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-small">
        <span className="text-muted tabular-nums">{formatDay(locale, day.at)}</span>
        {layers.map((layer) => (
          <span key={layer} className="flex items-center gap-1.5">
            <span className={cn("size-1.5 rounded-full bg-current", LAYER_TONE[layer])} />
            <span className="text-muted">{t(LAYER_NAME[layer])}</span>
            <Money
              currency="platinum"
              amount={priceOn(day, layer)}
              className="font-semibold text-body text-ink"
            />
            {layer === "trades" && <span className="text-muted tabular-nums">{day.trades}</span>}
          </span>
        ))}
        {!showsTrades && traded && (
          <span className="text-muted tabular-nums">{t("item-trades", { count: day.trades })}</span>
        )}
      </div>
      <div ref={setBox} className="relative">
        {width > 0 && (
          <svg
            width={width}
            height={height + GAP + TRADES}
            role="img"
            aria-label={t("item-trades", { count: day.trades })}
            className="block touch-none"
            onPointerMove={(event) => {
              const left = event.currentTarget.getBoundingClientRect().left;
              setPointed(nearestDay(days, (event.clientX - left) / (plot || 1)));
            }}
            onPointerLeave={() => setPointed(null)}
          >
            <defs>
              <linearGradient id={gradient} x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="currentColor" stopOpacity={0.22} />
                <stop offset="100%" stopColor="currentColor" stopOpacity={0} />
              </linearGradient>
            </defs>
            {ticks.map((tick) => (
              <g key={tick} className="text-dim">
                <line
                  x1={0}
                  x2={plot}
                  y1={y(tick)}
                  y2={y(tick)}
                  stroke="currentColor"
                  strokeOpacity={0.25}
                />
                <text
                  x={width}
                  y={y(tick)}
                  dy="0.32em"
                  textAnchor="end"
                  fill="currentColor"
                  className="text-caption tabular-nums"
                >
                  {formatCount(locale, tick)}
                </text>
              </g>
            ))}
            {mark !== undefined && mark > first && mark < first + span && (
              <line
                x1={((mark - first) / span) * plot}
                x2={((mark - first) / span) * plot}
                y1={0}
                y2={height + GAP + TRADES}
                stroke="currentColor"
                strokeDasharray="3 3"
                className="text-dim"
              />
            )}
            {showsTrades && area && tradeLine && (
              <g className={LAYER_TONE.trades}>
                <path d={area} fill={`url(#${gradient})`} />
                <path
                  d={tradeLine}
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={1.5}
                  strokeLinejoin="round"
                />
              </g>
            )}
            {layers
              .filter((layer) => layer !== "trades")
              .map((layer) => {
                const path = pathOf(layer);
                return (
                  path && (
                    <path
                      key={layer}
                      d={path}
                      fill="none"
                      stroke="currentColor"
                      strokeWidth={1.25}
                      strokeLinejoin="round"
                      className={LAYER_TONE[layer]}
                    />
                  )
                );
              })}
            {traded &&
              days.map((each, index) => {
                const tall = (each.trades / most) * TRADES;
                return (
                  <rect
                    key={each.at}
                    x={x(index) - bar / 2}
                    y={height + GAP + TRADES - tall}
                    width={bar}
                    height={tall}
                    rx={1}
                    fill="currentColor"
                    className={index === pointed ? "text-soft" : "text-dim/50"}
                  />
                );
              })}
            {pointed !== null && (
              <g>
                <line
                  x1={x(shown)}
                  x2={x(shown)}
                  y1={0}
                  y2={height}
                  stroke="currentColor"
                  strokeOpacity={0.5}
                  className="text-soft"
                />
                {layers.map((layer) => {
                  const price = priceOn(day, layer);
                  return (
                    price !== null && (
                      <circle
                        key={layer}
                        cx={x(shown)}
                        cy={y(price)}
                        r={3}
                        fill="currentColor"
                        className={LAYER_TONE[layer]}
                      />
                    )
                  );
                })}
              </g>
            )}
          </svg>
        )}
      </div>
      <div
        className="flex justify-between text-caption text-dim tabular-nums"
        style={{ paddingRight: AXIS }}
      >
        <span>{formatDay(locale, days[0].at)}</span>
        <span>{formatDay(locale, days[Math.floor((days.length - 1) / 2)].at)}</span>
        <span>{formatDay(locale, days[days.length - 1].at)}</span>
      </div>
    </div>
  );
}
