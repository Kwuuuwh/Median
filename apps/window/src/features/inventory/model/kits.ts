import type { Good, SetRow } from "@median/ipc";
import { partName } from "@/features/items/model/composition";
import { compareMeasures, type Sorting } from "@/shared/lib/sort";

/** One part of a set with the account's count of it. */
export type KitPart = {
  item: string;
  /** The part's name without the set's name. */
  name: string;
  image: string | null;
  held: number;
  /** Copies one set needs. */
  need: number;
  /** Market price of one copy; null while unknown. */
  price: number | null;
  ducats: number | null;
};

/** A set the account holds any part of: complete sets its parts make, and each part's count. */
export type Kit = {
  slug: string;
  /** The item the parts build. */
  item: string;
  name: string;
  image: string | null;
  vaulted: boolean;
  /** Complete sets the held parts make. */
  whole: number;
  parts: KitPart[];
  /** Market price of the complete set; null while unknown. */
  price: number | null;
};

/** Sort keys of the sets. */
export type KitOrder = "ready" | "platinum" | "missing";

/** Parts the account holds fewer of than one set needs. */
export function missingOf(kit: Kit): KitPart[] {
  return kit.parts.filter((part) => part.held < part.need);
}

/** Platinum cost of the copies missing for one set; null if any missing part is unpriced. */
export function missingCost(kit: Kit): number | null {
  let sum = 0;
  for (const part of missingOf(kit)) {
    if (part.price === null) return null;
    sum += part.price * (part.need - part.held);
  }
  return sum;
}

/** Every set the account holds a part of, with each part's name, count and price. */
export function kitsOf(
  sets: readonly SetRow[],
  held: ReadonlyMap<string, number>,
  good: (item: string) => Good | undefined,
  price: (item: string) => number | null,
  setPrice: (slug: string) => number | null,
): Kit[] {
  return sets.flatMap((set): Kit[] => {
    if (!set.built || set.members.length === 0) return [];
    if (!set.members.some((member) => (held.get(member) ?? 0) > 0)) return [];
    const name = good(set.built)?.name ?? set.name;
    const parts = set.members.map((member, at) => ({
      item: member,
      name: partName(good(member)?.name ?? member, name),
      image: good(member)?.image ?? null,
      held: held.get(member) ?? 0,
      need: set.per_set[at] ?? 1,
      price: price(member),
      ducats: good(member)?.ducats ?? null,
    }));
    return [
      {
        slug: set.slug,
        item: set.built,
        name,
        image: good(set.built)?.image ?? null,
        vaulted: set.vaulted ?? false,
        whole: Math.min(...parts.map((part) => Math.floor(part.held / part.need))),
        parts,
        price: setPrice(set.slug),
      },
    ];
  });
}

/** Fraction of the set's parts held in full, from 0 to 1. */
function readiness(kit: Kit): number {
  return kit.parts.filter((part) => part.held >= part.need).length / kit.parts.length;
}

/** Initial direction of a newly picked set sort key: ascending for `missing`, else descending. */
export function kitWay(by: KitOrder): boolean {
  return by !== "missing";
}

/** Sets sorted by the chosen key and direction; unknown values last, ties by higher set price. */
export function sortKits(kits: readonly Kit[], sorting: Sorting<KitOrder>): Kit[] {
  const { by, descending } = sorting;
  const measure = (kit: Kit): number | null =>
    by === "platinum" ? kit.price : by === "missing" ? missingCost(kit) : readiness(kit);
  return [...kits].sort(
    (a, b) =>
      compareMeasures(measure(a), measure(b), descending) ||
      compareMeasures(a.price, b.price, true),
  );
}
