import type { RivenBuy } from "@median/ipc";

/** Totals of the agent's riven purchases and sales over a period, in platinum and counts. */
export type RivenFigures = {
  bought: number;
  spent: number;
  sold: number;
  earned: number;
  /** Sale prices minus purchase prices of the rivens sold in the period. */
  profit: number;
  /** Platinum spent on rivens not sold yet, regardless of when they were bought. */
  held: number;
};

/** Riven totals since Unix second `since`; `0` covers all purchases and sales. */
export function rivenFigures(buys: readonly RivenBuy[], since: number): RivenFigures {
  const bought = buys.filter((buy) => buy.bought >= since);
  const sold = buys.filter((buy) => buy.sold !== null && buy.sold >= since);
  const sum = (values: number[]) => values.reduce((total, value) => total + value, 0);
  return {
    bought: bought.length,
    spent: sum(bought.map((buy) => buy.price)),
    sold: sold.length,
    earned: sum(sold.map((buy) => buy.sale ?? 0)),
    profit: sum(sold.map((buy) => (buy.sale ?? 0) - buy.price)),
    held: sum(buys.filter((buy) => buy.sold === null).map((buy) => buy.price)),
  };
}
