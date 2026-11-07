import type { Dealt, Position } from "@median/ipc";
import { DAY } from "@/shared/lib/time";

/** Totals of a set of exchanges, in platinum and counts. */
export type Summary = {
  /** Sum of sale margins: sale price minus the copies' average cost. */
  margin: number;
  /** Number of sales with a known cost, so with a margin. */
  margined: number;
  taken: number;
  sales: number;
  spent: number;
  buys: number;
};

/** One day: its start in Unix seconds, platinum from sales, spent on buys, and sale margin. */
export type Day = { starts: number; taken: number; spent: number; margin: number };

/** One item's sales combined: total margin and number of sales. */
export type Earner = {
  item: string;
  name: string;
  image: string | null;
  margin: number;
  sales: number;
};

/** Platinum of an exchange: price times quantity. */
function movedBy(deal: Dealt): number {
  return deal.platinum * deal.quantity;
}

/** Exchanges that filled orders the agent placed. */
export function byAgent(deals: readonly Dealt[]): Dealt[] {
  return deals.filter((deal) => deal.agent);
}

/** Totals of the exchanges. */
export function summarize(deals: readonly Dealt[]): Summary {
  const sales = deals.filter((deal) => deal.selling);
  const buys = deals.filter((deal) => !deal.selling);
  const margins = sales.flatMap((deal) => (deal.margin === null ? [] : [deal.margin]));
  return {
    margin: margins.reduce((sum, margin) => sum + margin, 0),
    margined: margins.length,
    taken: sales.reduce((sum, deal) => sum + movedBy(deal), 0),
    sales: sales.length,
    spent: buys.reduce((sum, deal) => sum + movedBy(deal), 0),
    buys: buys.length,
  };
}

/** Per-day totals over the last `days` UTC days up to `now`; a day without exchanges is zero. */
export function daily(deals: readonly Dealt[], days: number, now: number): Day[] {
  const today = Math.floor(now / DAY) * DAY;
  const first = today - (days - 1) * DAY;
  const series: Day[] = Array.from({ length: days }, (_, at) => ({
    starts: first + at * DAY,
    taken: 0,
    spent: 0,
    margin: 0,
  }));
  for (const deal of deals) {
    const day = series[Math.floor((deal.at - first) / DAY)];
    if (!day) continue;
    if (deal.selling) {
      day.taken += movedBy(deal);
      day.margin += deal.margin ?? 0;
    } else {
      day.spent += movedBy(deal);
    }
  }
  return series;
}

/** Totals of one item group. */
export type Grouped = Summary & { group: string };

/** Item groups of the statistics, in the order used when totals are equal. */
const GROUPS = ["mod", "arcane", "prime", "set", "relic", "riven", "other"] as const;

/**
 * Totals of every item group, highest margin first; a group without exchanges is zero, and an
 * unknown group counts as `other`.
 */
export function byGroup(deals: readonly Dealt[]): Grouped[] {
  const groups = new Map<string, Dealt[]>(GROUPS.map((group) => [group, []]));
  for (const deal of deals) {
    const group = groups.has(deal.group) ? deal.group : "other";
    groups.get(group)?.push(deal);
  }
  return [...groups.entries()]
    .map(([group, each]) => ({ group, ...summarize(each) }))
    .sort((a, b) => b.margin - a.margin || b.taken - a.taken);
}

/** Markup in percent of the sales with a known cost (margin over cost); null without any. */
export function markup(deals: readonly Dealt[]): number | null {
  let margin = 0;
  let cost = 0;
  for (const deal of deals) {
    if (!deal.selling || deal.margin === null) continue;
    margin += deal.margin;
    cost += movedBy(deal) - deal.margin;
  }
  return cost > 0 ? Math.round((margin / cost) * 100) : null;
}

/**
 * Cost, current value and copy count of the unsold positions; a position without an in-game price
 * is valued at its cost.
 */
export function unsold(positions: readonly Position[]): {
  cost: number;
  worth: number;
  copies: number;
} {
  return positions.reduce(
    (sum, position) => ({
      cost: sum.cost + position.basis * position.quantity,
      worth: sum.worth + (position.ingame ?? position.basis) * position.quantity,
      copies: sum.copies + position.quantity,
    }),
    { cost: 0, worth: 0, copies: 0 },
  );
}

/** Up to `most` items with the highest total sale margin. */
export function earners(deals: readonly Dealt[], most: number): Earner[] {
  const byItem = new Map<string, Earner>();
  for (const deal of deals) {
    if (!deal.selling || deal.margin === null) continue;
    const key = deal.item || deal.name;
    const earner = byItem.get(key) ?? {
      item: deal.item,
      name: deal.name,
      image: deal.image,
      margin: 0,
      sales: 0,
    };
    earner.margin += deal.margin;
    earner.sales += 1;
    byItem.set(key, earner);
  }
  return [...byItem.values()].sort((a, b) => b.margin - a.margin).slice(0, most);
}
