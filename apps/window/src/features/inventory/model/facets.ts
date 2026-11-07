import type { HeldLine, InventoryTab, LineOrder } from "./lines";

/** A yes/no filter for inventory lines. */
export type Facet =
  | "built"
  | "many"
  | "vaulted"
  | "listed"
  | "prime"
  | "favourite"
  | "equipped"
  | "upgraded"
  | "tradable"
  | "spare";

/** Active filters, each mapped to the required value: yes or no. */
export type Facets = ReadonlyMap<Facet, boolean>;

/** Line flags that come from outside the inventory: market orders and favourites. */
export type Marks = {
  /** Whether the account has a sell order for the line. */
  listed: (line: HeldLine) => boolean;
  favourite: (line: HeldLine) => boolean;
};

/** Filters each tab offers, in display order. */
export const TAB_FACETS: Record<InventoryTab, readonly Facet[]> = {
  parts: ["spare", "built", "many", "vaulted", "listed", "prime", "favourite"],
  mods: ["many", "listed", "equipped", "upgraded", "favourite"],
  arcanes: ["spare", "many", "listed", "equipped", "upgraded", "favourite"],
  sets: ["built", "many", "vaulted", "listed", "favourite"],
  other: ["many", "tradable", "listed", "prime", "favourite"],
};

/** Sort keys each tab offers; the first is the default. */
export const TAB_ORDERS: Record<InventoryTab, readonly LineOrder[]> = {
  parts: ["platinum", "name", "ducats", "count"],
  mods: ["platinum", "name", "count", "rank"],
  arcanes: ["platinum", "name", "count", "rank"],
  sets: ["platinum", "name", "ducats", "count"],
  other: ["platinum", "name", "count"],
};

/** The line's value for a filter. */
function answer(facet: Facet, line: HeldLine, marks: Marks): boolean {
  switch (facet) {
    case "built":
      return line.built !== null;
    case "many":
      return line.count > 1;
    case "vaulted":
      return line.vaulted;
    case "listed":
      return marks.listed(line);
    case "prime":
      return line.prime;
    case "favourite":
      return marks.favourite(line);
    case "equipped":
      return line.equipped;
    case "upgraded":
      return (line.rank ?? 0) > 0;
    case "tradable":
      return line.tradable;
    case "spare":
      return line.spare > 0;
  }
}

/** Whether a line matches the required value of every active filter. */
export function passes(line: HeldLine, facets: Facets, marks: Marks): boolean {
  for (const [facet, wanted] of facets) {
    if (answer(facet, line, marks) !== wanted) return false;
  }
  return true;
}
