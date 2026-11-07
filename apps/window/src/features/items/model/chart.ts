import type { Curve } from "@median/ipc";
import { DAY } from "@/shared/lib/time";

/** Chart series: closed-trade price, and the best open sell and buy order prices. */
export type Layer = "trades" | "selling" | "buying";

/** One market day: closed-trade price and count, lowest sell order and highest buy order. */
export type Day = {
  at: number;
  platinum: number | null;
  trades: number;
  selling: number | null;
  buying: number | null;
};

/**
 * The last `count` UTC days up to `now`, oldest first, with each series value on the day it was
 * recorded; a day without data has null prices and 0 trades.
 */
export function daysOf(
  curve: Pick<Curve, "closed" | "selling" | "buying">,
  count: number,
  now: number,
): Day[] {
  const today = Math.floor(now / DAY) * DAY;
  const days = Array.from(
    { length: count },
    (_, index): Day => ({
      at: today - (count - 1 - index) * DAY,
      platinum: null,
      trades: 0,
      selling: null,
      buying: null,
    }),
  );
  const byStart = new Map(days.map((day) => [day.at, day]));
  const day = (at: number) => byStart.get(Math.floor(at / DAY) * DAY);
  for (const spot of curve.closed) {
    const found = day(spot.at);
    if (found) Object.assign(found, { platinum: spot.platinum, trades: spot.weight });
  }
  for (const standing of curve.selling) {
    const found = day(standing.at);
    if (found) found.selling = standing.best;
  }
  for (const standing of curve.buying) {
    const found = day(standing.at);
    if (found) found.buying = standing.best;
  }
  return days;
}

/** Average trades per day over the 7 full days before today; null when none of them had trades. */
export function dailyTrades(curve: Pick<Curve, "closed">, now: number): number | null {
  const today = Math.floor(now / DAY) * DAY;
  const week = curve.closed.filter((spot) => spot.at >= today - 7 * DAY && spot.at < today);
  return week.length > 0 ? week.reduce((sum, spot) => sum + spot.weight, 0) / 7 : null;
}

/** Series to draw: sell and buy orders when any day has them, else the closed-trade price. */
export function layersOf(days: readonly Day[]): readonly Layer[] {
  return days.some((day) => day.selling !== null || day.buying !== null)
    ? ["selling", "buying"]
    : ["trades"];
}

/** A day's price in one series. */
export function priceOn(day: Day, layer: Layer): number | null {
  switch (layer) {
    case "trades":
      return day.platinum;
    case "selling":
      return day.selling;
    case "buying":
      return day.buying;
  }
}

/**
 * Price axis ticks between `low` and `high` at a round step (1, 2 or 5 times a power of ten, at
 * least 1 platinum); `[low]` when the range is empty.
 */
export function priceTicks(low: number, high: number, count = 4): number[] {
  if (high <= low) return [low];
  const rough = (high - low) / count;
  const power = 10 ** Math.floor(Math.log10(rough));
  const step = Math.max(
    1,
    [1, 2, 5, 10].map((each) => each * power).find((each) => each >= rough) ?? power,
  );
  const ticks: number[] = [];
  for (let tick = Math.ceil(low / step) * step; tick <= high; tick += step) ticks.push(tick);
  return ticks;
}

/** Index of the day nearest a position along the chart, `share` from 0 to 1, clamped. */
export function nearestDay(days: readonly Day[], share: number): number {
  return Math.min(days.length - 1, Math.max(0, Math.round(share * (days.length - 1))));
}

/** Index of the last day with a price in any of `layers`; -1 when there is none. */
export function lastPriced(days: readonly Day[], layers: readonly Layer[]): number {
  for (let index = days.length - 1; index >= 0; index--) {
    if (layers.some((layer) => priceOn(days[index], layer) !== null)) return index;
  }
  return -1;
}
