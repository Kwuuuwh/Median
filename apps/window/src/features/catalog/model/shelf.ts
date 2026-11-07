import type { Collectible } from "@median/ipc";
import { nameCollator } from "@/shared/lib/sort";

/** The catalog's yes/no filters, in the order the menu offers them. */
export const CATALOG_FACETS = [
  "held",
  "mastered",
  "ready",
  "reachable",
  "vaulted",
  "relics",
  "favourite",
] as const;

/** A yes/no filter of the catalog. */
export type CatalogFacet = (typeof CATALOG_FACETS)[number];

/** Active filters, each mapped to the required value: yes or no. */
export type CatalogFacets = ReadonlyMap<CatalogFacet, boolean>;

/** What the catalog is narrowed to. */
export type Narrowed = {
  /** A class slug; `null` for every class. */
  className: string | null;
  /** Whether the search lets the item through. */
  matches: (item: string, name: string) => boolean;
  facets: CatalogFacets;
  favourite: (item: string) => boolean;
  locale: string;
};

/**
 * The row's value for a filter; `null` when the row cannot say, as an item that gives no mastery
 * cannot say whether it is mastered.
 */
function answer(
  facet: CatalogFacet,
  row: Collectible,
  favourite: (item: string) => boolean,
): boolean | null {
  switch (facet) {
    case "held":
      return row.owned > 0;
    case "mastered":
      return row.mastered;
    case "ready":
      return row.ready;
    case "reachable":
      return row.reachable;
    case "vaulted":
      return row.vaulted;
    case "relics":
      return row.relics;
    case "favourite":
      return favourite(row.item);
  }
}

/**
 * The rows the class, the search and the filters let through, by name. A row that cannot answer
 * an active filter is held back by it either way.
 */
export function shelve(rows: readonly Collectible[], narrowed: Narrowed): Collectible[] {
  const { compare } = nameCollator(narrowed.locale);
  return rows
    .filter(
      (row) =>
        (narrowed.className === null || row.class === narrowed.className) &&
        narrowed.matches(row.item, row.name) &&
        [...narrowed.facets].every(
          ([facet, wanted]) => answer(facet, row, narrowed.favourite) === wanted,
        ),
    )
    .sort((one, other) => compare(one.name, other.name));
}
