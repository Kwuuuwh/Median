import type { Card, Drop, Reward } from "@median/ipc";

/** One relic that drops an item, with all its refinements combined. */
export type RelicSource = {
  name: string;
  image: string | null;
  rarity: string;
  /** The item's drop chance at each refinement, in tenths of a percent. */
  chance: Partial<Record<string, number>>;
  /** Item keys of the relic's refinements. */
  items: string[];
  /** Item key the relic's card opens: the intact refinement, else the first one found. */
  opens: string;
  held: number;
  vaulted: boolean;
  /** Whether a vendor currently sells the relic. */
  offered: boolean;
};

/**
 * Average runs to get a reward with this chance in tenths of a percent, rounded up: 100 (10%)
 * takes 10; infinite for 0.
 */
export function runsFor(chance: number): number {
  return chance > 0 ? Math.ceil(1000 / chance) : Number.POSITIVE_INFINITY;
}

/**
 * Relics that drop an item, one per relic: held first, then obtainable, then by chance at
 * `refinement`, then by name.
 */
export function relicSources(
  found: readonly Reward[],
  held: (item: string) => number,
  vaulted: (item: string) => boolean,
  refinement: string,
  offered: (item: string) => boolean = () => false,
): RelicSource[] {
  const byName = new Map<string, RelicSource>();
  for (const reward of found) {
    const at = reward.refinement ?? "intact";
    const source = byName.get(reward.name) ?? {
      name: reward.name,
      image: reward.image,
      rarity: reward.rarity,
      chance: {},
      items: [],
      opens: reward.item,
      held: 0,
      vaulted: false,
      offered: false,
    };
    source.chance[at] = reward.chance;
    source.items.push(reward.item);
    if (at === "intact") source.opens = reward.item;
    source.held += held(reward.item);
    source.vaulted ||= vaulted(reward.item);
    source.offered ||= offered(reward.item);
    byName.set(reward.name, source);
  }
  return [...byName.values()].sort(
    (a, b) =>
      Number(b.held > 0) - Number(a.held > 0) ||
      Number(a.vaulted && !a.offered) - Number(b.vaulted && !b.offered) ||
      (b.chance[refinement] ?? 0) - (a.chance[refinement] ?? 0) ||
      a.name.localeCompare(b.name),
  );
}

/** Mission drops, highest chance first. */
export function dropSources(drops: readonly Drop[]): Drop[] {
  return [...drops].sort((a, b) => b.chance - a.chance);
}

/** Whether a relic is obtainable: held, not vaulted, or sold by a vendor now. */
export function isReachable(source: RelicSource): boolean {
  return source.held > 0 || !source.vaulted || source.offered;
}

/** Source kinds in the order an item card shows them. */
export const SOURCE_KINDS = [
  "relic",
  "node",
  "bounty",
  "key",
  "transient",
  "sortie",
  "enemy",
  "event",
  "vendor",
  "lab",
] as const;

/** Number of sources of each kind on a card; kinds without sources are omitted. */
export function sourceCounts(card: Card): Map<string, number> {
  const counts = new Map<string, number>();
  const add = (kind: string, count: number) => {
    if (count > 0) counts.set(kind, (counts.get(kind) ?? 0) + count);
  };
  add("relic", new Set(card.found.map((reward) => reward.name)).size);
  for (const drop of card.drops) add(drop.kind, 1);
  add("vendor", card.sold.length);
  add("lab", card.research.length);
  return counts;
}
