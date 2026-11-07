import type { HaulLine } from "@median/ipc";

/** One item gained in the haul, with the value of its copies. */
export type HaulRow = {
  item: string;
  variant: string;
  name: string;
  image: string | null;
  count: number;
  /** Expected platinum price of one copy; null while no price is known. */
  each: number | null;
  /** Expected platinum price of all copies together; null while no price is known. */
  platinum: number | null;
  /** Ducat value of all copies together. */
  ducats: number;
};

/**
 * Haul items worth platinum or ducats, plus items unknown to the catalog; highest platinum first,
 * then highest ducats.
 */
export function haulRows(lines: readonly HaulLine[]): HaulRow[] {
  return lines
    .filter((line) => !line.known || (line.worth ?? 0) > 0 || (line.ducats ?? 0) > 0)
    .map((line) => ({
      item: line.item,
      variant: line.variant,
      name: line.name,
      image: line.image,
      count: line.count,
      each: line.worth,
      platinum: line.worth === null ? null : line.worth * line.count,
      ducats: (line.ducats ?? 0) * line.count,
    }))
    .sort((a, b) => (b.platinum ?? 0) - (a.platinum ?? 0) || b.ducats - a.ducats);
}

/** Total platinum and ducat value of the haul rows; unpriced rows add no platinum. */
export function haulTotals(rows: readonly HaulRow[]): { platinum: number; ducats: number } {
  return rows.reduce(
    (sum, row) => ({
      platinum: sum.platinum + (row.platinum ?? 0),
      ducats: sum.ducats + row.ducats,
    }),
    { platinum: 0, ducats: 0 },
  );
}
