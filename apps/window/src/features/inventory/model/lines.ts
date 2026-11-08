import type { Detail, Good, Pair, Piece, Priced, SetRow } from "@median/ipc";
import { nameFromKey } from "@/features/items/model/name";
import { compareMeasures, compareNames, nameCollator, type Sorting } from "@/shared/lib/sort";

/** Inventory tab. */
export type InventoryTab = "parts" | "mods" | "arcanes" | "sets" | "other";

/** Sort keys of inventory lines. */
export type LineOrder = "name" | "platinum" | "ducats" | "count" | "rank";

/** Progress with the item a line builds: `mastered`, or `crafted` (built and held). */
type Built = "mastered" | "crafted";

/**
 * One inventory line: an item variant the game distinguishes, with its catalog, market and
 * account data.
 */
export type HeldLine = {
  item: string;
  variant: string;
  name: string;
  image: string | null;
  known: boolean;
  kind: string;
  /** Catalog class of the item's kind. */
  group: string;
  count: number;
  detail: Detail | null;
  /** Rank of the copies, for an item with ranks. */
  rank: number | null;
  /** Highest rank a copy can reach, for an item with ranks. */
  maxRank: number | null;
  /** Items a copy is installed on; null when the game reports only that it is installed. */
  fitted: string[] | null;
  equipped: boolean;
  tradable: boolean;
  prime: boolean;
  vaulted: boolean;
  ducats: number | null;
  built: Built | null;
  /** Price of one copy in platinum. */
  platinum: number | null;
  /** Price of one copy at the top rank, for an item with ranks. */
  topped: number | null;
  /**
   * Rank-zero copies still needed to fuse one copy to the top rank; null once a copy is at the top
   * rank, or for an item that is not fused.
   */
  toMax: number | null;
  /**
   * Copies the account can trade away: for an arcane, all but one top-rank copy; for a set part,
   * the copies beyond one build.
   */
  spare: number;
  /** The vendor's arcane collection that can roll the item. */
  collection: string | null;
  /** The line's market pair, when warframe.market lists it. */
  pair: Pair | null;
};

/** A piece that may list the items its copies are installed on, when the backend sends them. */
type Fitted = Piece & { fitted?: string[] };

/** Inputs the inventory lines are built from. */
export type LineSources = {
  pieces: readonly Piece[];
  goods: ReadonlyMap<string, Good>;
  /** Class of each catalog kind. */
  classes: ReadonlyMap<string, string>;
  sets: readonly SetRow[];
  held: ReadonlyMap<string, number>;
  price: (item: string, variant: string) => Priced | undefined;
  /** Price of a complete set, by its slug. */
  setPrice: (slug: string) => Priced | undefined;
};

/** `group` value of a complete set's line. */
export const SET_GROUP = "?set";

/** Progress with the item a set builds: mastered, else crafted when the built item is held. */
function builtOf(mastered: boolean, held: number): Built | null {
  if (mastered) return "mastered";
  return held > 0 ? "crafted" : null;
}

/** Copies of a set part beyond one build of the item; all copies once the item is built. */
function partSpare(count: number, need: number, built: Built | null): number {
  return built ? count : Math.max(0, count - need);
}

/** Inventory lines with a positive count, then one line per set the held parts complete. */
export function heldLines(sources: LineSources): HeldLine[] {
  // Only the main blueprint carries the mastered flag, so a set counts as mastered when any of
  // its members has it.
  const builtBy = new Map<string, string>();
  const needOf = new Map<string, number>();
  const mastered = new Set<string>();
  for (const set of sources.sets) {
    const whole = set.built;
    if (!whole) continue;
    set.members.forEach((member, at) => {
      builtBy.set(member, whole);
      needOf.set(member, set.per_set[at] ?? 1);
    });
    if (set.members.some((member) => sources.goods.get(member)?.mastered)) mastered.add(whole);
  }
  const items = sources.pieces.flatMap((piece): HeldLine[] => {
    if (piece.count <= 0) return [];
    const good = sources.goods.get(piece.item);
    const known = good?.known ?? false;
    const priced = sources.price(piece.item, piece.variant);
    const made = builtBy.get(piece.item);
    const built = made
      ? builtOf(mastered.has(made), sources.held.get(made) ?? 0)
      : good?.mastered || mastered.has(piece.item)
        ? "mastered"
        : null;
    const detail = piece.detail;
    return [
      {
        item: piece.item,
        variant: piece.variant,
        name: known && good ? good.name : nameFromKey(piece.item),
        image: good?.image ?? null,
        known,
        kind: good?.kind ?? "",
        group: sources.classes.get(good?.kind ?? "") ?? "",
        count: piece.count,
        detail,
        rank: detail && detail.kind !== "stars" ? detail.rank : null,
        maxRank: priced?.maxRank ?? null,
        fitted: (piece as Fitted).fitted ?? (piece.equipped ? null : []),
        equipped: piece.equipped,
        tradable: good?.tradable ?? false,
        prime: good?.prime ?? false,
        vaulted: good?.vaulted ?? false,
        ducats: good?.ducats ?? null,
        collection: good?.collection ?? null,
        built,
        platinum: priced?.worth ?? null,
        topped: priced?.topped ?? null,
        toMax: null,
        spare: made ? partSpare(piece.count, needOf.get(piece.item) ?? 1, built) : 0,
        pair: priced ? { slug: priced.slug, key: priced.key } : null,
      },
    ];
  });
  const sets = sources.sets.flatMap((set): HeldLine[] => {
    if (!set.built || set.members.length === 0) return [];
    const need = (at: number) => set.per_set[at] ?? 1;
    const count = Math.min(
      ...set.members.map((member, at) => Math.floor((sources.held.get(member) ?? 0) / need(at))),
    );
    if (count <= 0) return [];
    const good = sources.goods.get(set.built);
    const priced = sources.setPrice(set.slug);
    const ducats = set.members.reduce(
      (sum, member, at) => sum + (sources.goods.get(member)?.ducats ?? 0) * need(at),
      0,
    );
    return [
      {
        item: set.built,
        variant: "",
        name: good?.name ?? set.name,
        image: good?.image ?? null,
        known: true,
        kind: "",
        group: SET_GROUP,
        count,
        detail: null,
        rank: null,
        maxRank: null,
        fitted: [],
        equipped: false,
        tradable: true,
        prime: good?.prime ?? false,
        vaulted: set.vaulted ?? false,
        ducats: ducats > 0 ? ducats : null,
        collection: null,
        built: builtOf(mastered.has(set.built), sources.held.get(set.built) ?? 0),
        platinum: priced?.worth ?? null,
        topped: null,
        toMax: null,
        spare: 0,
        pair: priced ? { slug: priced.slug, key: priced.key } : null,
      },
    ];
  });
  return [...fused(items), ...sets];
}

/** Rank-zero copies fused into one copy at the rank: (rank + 1)(rank + 2) / 2, so 21 at rank 5. */
function fusedAt(rank: number): number {
  return ((rank + 1) * (rank + 2)) / 2;
}

/**
 * Arcane lines with `toMax` set while no copy is at the top rank, or with `spare` counting the
 * copies beyond one top-rank copy once one is.
 */
function fused(lines: readonly HeldLine[]): HeldLine[] {
  const pooled = new Map<string, number>();
  const topped = new Set<string>();
  for (const line of lines) {
    if (line.group !== "arcane" || line.maxRank === null) continue;
    const rank = line.rank ?? 0;
    pooled.set(line.item, (pooled.get(line.item) ?? 0) + fusedAt(rank) * line.count);
    if (rank >= line.maxRank) topped.add(line.item);
  }
  return lines.map((line) => {
    if (line.group !== "arcane" || line.maxRank === null) return line;
    if (!topped.has(line.item)) {
      const toMax = Math.max(0, fusedAt(line.maxRank) - (pooled.get(line.item) ?? 0));
      return { ...line, toMax };
    }
    return { ...line, spare: (line.rank ?? 0) >= line.maxRank ? line.count - 1 : line.count };
  });
}

/** Kinds shown on their own pages instead of the inventory: relics and rivens. */
const ELSEWHERE = new Set(["relic", "riven"]);

/** Classes of built equipment: what the arsenal, the ship and the railjack hold. */
const EQUIPMENT = new Set([
  "warframe",
  "primary",
  "secondary",
  "melee",
  "exalted",
  "archwing-weapon",
  "companion",
  "companion-weapon",
  "modular",
  "railjack",
  "ship",
]);

/** The tab a line belongs to; null for a line shown on its own page and for built equipment. */
export function tabOf(line: HeldLine, members: ReadonlySet<string>): InventoryTab | null {
  if (ELSEWHERE.has(line.kind) || EQUIPMENT.has(line.group)) return null;
  if (line.group === SET_GROUP) return "sets";
  if (!line.known) return "other";
  if (line.group === "mod") return "mods";
  if (line.group === "arcane") return "arcanes";
  if (members.has(line.item)) return "parts";
  return "other";
}

/**
 * Whether a copy's price lies within the bounds, using the top-rank price for an arcane; a null
 * bound is open, and an unpriced line fails any set bound.
 */
export function inPriceRange(line: HeldLine, from: number | null, to: number | null): boolean {
  return inRange(
    line.group === "arcane" ? (line.topped ?? line.platinum) : line.platinum,
    from,
    to,
  );
}

/** Whether a price lies within the bounds; a null bound is open, a null price fails a set bound. */
export function inRange(price: number | null, from: number | null, to: number | null): boolean {
  if (from === null && to === null) return true;
  if (price === null) return false;
  return (from === null || price >= from) && (to === null || price <= to);
}

/** Total platinum and ducat value of the lines' copies, or of the spare copies only. */
export function totals(
  lines: readonly HeldLine[],
  spareOnly = false,
): { platinum: number; ducats: number } {
  let platinum = 0;
  let ducats = 0;
  for (const line of lines) {
    const copies = spareOnly ? line.spare : line.count;
    platinum += (line.platinum ?? 0) * copies;
    ducats += (line.ducats ?? 0) * copies;
  }
  return { platinum, ducats };
}

/** Lines sorted by the chosen key and direction; unknown values last, ties by name. */
export function sortLines(
  lines: readonly HeldLine[],
  sorting: Sorting<LineOrder>,
  locale: string,
): HeldLine[] {
  const { by, descending } = sorting;
  const collator = nameCollator(locale);
  const measure = (line: HeldLine): number | null =>
    by === "platinum"
      ? line.platinum
      : by === "ducats"
        ? line.ducats
        : by === "rank"
          ? line.rank
          : line.count;
  return [...lines].sort((a, b) =>
    by === "name"
      ? compareNames(collator, a.name, b.name, descending)
      : compareMeasures(measure(a), measure(b), descending) || collator.compare(a.name, b.name),
  );
}

/** A word with quote marks and colons removed. */
function bare(word: string): string {
  return word.replace(/[«»"“”:]/g, "");
}

/**
 * Short label of each name: its words without the leading and trailing words that all names share;
 * the full name when nothing is shared or nothing is left.
 */
export function shortNames(names: readonly string[]): Map<string, string> {
  const split = names.map((name) => name.split(/\s+/).filter(Boolean));
  const shared = (pick: (words: string[], at: number) => string): number => {
    let count = 0;
    while (
      split.length > 1 &&
      split.every(
        (words) =>
          words.length > count + 1 && bare(pick(words, count)) === bare(pick(split[0], count)),
      )
    ) {
      count += 1;
    }
    return count;
  };
  const head = shared((words, at) => words[at]);
  const tail = shared((words, at) => words[words.length - 1 - at]);
  return new Map(
    names.map((name, at) => {
      const words = split[at].slice(head, Math.max(head, split[at].length - tail));
      return [name, (head + tail > 0 && words.map(bare).join(" ").trim()) || name];
    }),
  );
}

/** Copies free to sell: an equipped line keeps one copy, which serves every item it is on. */
export function freeCount(line: HeldLine): number {
  return line.equipped ? Math.max(0, line.count - 1) : line.count;
}
