import type { Masterable } from "@median/ipc";

/** Categories of items that give mastery, in page order. */
export const SHELVES = [
  "warframe",
  "primary",
  "secondary",
  "melee",
  "modular",
  "archwing",
  "companion",
  "companion-weapon",
] as const;

export type Shelf = (typeof SHELVES)[number];

/** Category of an item; null for a class outside every category. */
function shelfOf(item: Pick<Masterable, "class" | "kind">): Shelf | null {
  switch (item.class) {
    case "warframe":
      return item.kind === "warframe" ? "warframe" : "archwing";
    case "archwing-weapon":
    case "railjack":
      return "archwing";
    case "primary":
    case "secondary":
    case "melee":
    case "modular":
    case "companion":
    case "companion-weapon":
      return item.class;
    default:
      return null;
  }
}

/** Yes/no filters of the mastery items. */
export type MasteryFacet = "planned" | "held" | "reachable" | "prime" | "mastered";

/** Filters the mastery list offers, in display order. */
export const MASTERY_FACETS: readonly MasteryFacet[] = [
  "planned",
  "held",
  "reachable",
  "prime",
  "mastered",
];

/** Category, search, filters and planned items that narrow the mastery list. */
export type MasteryView = {
  shelf: Shelf | null;
  search: (item: Masterable) => boolean;
  facets: ReadonlyMap<MasteryFacet, boolean>;
  planned: ReadonlySet<string>;
};

/** Whether the item matches all active filters; mastered items only when `mastered` is yes. */
function passes(item: Masterable, rank: number, view: MasteryView) {
  const answer = (facet: MasteryFacet, value: boolean) => {
    const asked = view.facets.get(facet);
    return asked === undefined || asked === value;
  };
  return (
    answer("planned", view.planned.has(item.item)) &&
    answer("held", item.owned) &&
    answer("reachable", item.required <= rank) &&
    answer("prime", item.prime) &&
    (view.facets.get("mastered") ?? false) === (item.stage === "mastered")
  );
}

/** Items that pass the view's category, search and filters, sorted by name. */
export function shownItems(
  items: readonly Masterable[],
  rank: number,
  view: MasteryView,
): Masterable[] {
  return items
    .filter(
      (item) =>
        (view.shelf === null || shelfOf(item) === view.shelf) &&
        view.search(item) &&
        passes(item, rank, view),
    )
    .sort((a, b) => a.name.localeCompare(b.name));
}

/** Number of items not yet mastered in each category. */
export function shelfCounts(items: readonly Masterable[]): Map<Shelf, number> {
  const counts = new Map<Shelf, number>();
  for (const item of items) {
    const shelf = shelfOf(item);
    if (shelf === null || item.stage === "mastered") continue;
    counts.set(shelf, (counts.get(shelf) ?? 0) + 1);
  }
  return counts;
}
