import type { Listing } from "@median/ipc";

/** An open order as the item card shows it. */
export type Order = Listing;

/** Order sort keys in priority order: an earlier key decides first, whenever it was picked. */
const ORDER_KEYS = ["status", "reputation", "price", "quantity"] as const;
export type OrderKey = (typeof ORDER_KEYS)[number];

/** Ranks the order book of a ranked item can be filtered to. */
export const RANKS = ["zero", "max"] as const;
export type RankPick = (typeof RANKS)[number];

/** Whether an order is at the picked rank, 0 or `maxRank`; always true for an unranked item. */
export function atRank(order: Order, pick: RankPick, maxRank: number | null): boolean {
  if (maxRank === null) return true;
  return order.rank === (pick === "max" ? maxRank : 0);
}

/** Direction of each sort key, `true` for descending; an absent key does not sort. */
export type OrderSort = Partial<Record<OrderKey, boolean>>;

/** Sort weight of a player status: in game 2, online 1, anything else 0. */
function presence(status: string): number {
  if (status === "ingame") return 2;
  if (status === "online") return 1;
  return 0;
}

function measure(order: Order, key: OrderKey): number {
  switch (key) {
    case "status":
      return presence(order.status);
    case "reputation":
      return order.reputation;
    case "price":
      return order.platinum;
    case "quantity":
      return order.quantity;
  }
}

/** Orders sorted by the active keys in priority order; full ties by most recent update first. */
export function sortOrders(orders: readonly Order[], sort: OrderSort): Order[] {
  return [...orders].sort((a, b) => {
    for (const key of ORDER_KEYS) {
      const descending = sort[key];
      if (descending === undefined) continue;
      const diff = measure(a, key) - measure(b, key);
      if (diff !== 0) return descending ? -diff : diff;
    }
    return b.updated - a.updated;
  });
}

/** The sort after a key's header is clicked: the key cycles off, ascending, descending, off. */
export function pickKey(sort: OrderSort, key: OrderKey): OrderSort {
  const { [key]: descending, ...rest } = sort;
  if (descending === undefined) return { ...rest, [key]: false };
  if (!descending) return { ...rest, [key]: true };
  return rest;
}

/**
 * Default sort of a side, as on warframe.market: in-game players first, then the best price
 * (lowest for sell orders, highest for buy orders).
 */
export function firstSort(selling: boolean): OrderSort {
  return { status: true, price: !selling };
}
