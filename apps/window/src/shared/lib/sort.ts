/** A list's sort key and direction. */
export type Sorting<K extends string> = { by: K; descending: boolean };

/** Compares two values in the sort direction; null sorts last in both directions. */
export function compareMeasures(
  one: number | null,
  other: number | null,
  descending: boolean,
): number {
  if (one === other) return 0;
  if (one === null) return 1;
  if (other === null) return -1;
  return descending ? other - one : one - other;
}

const collators = new Map<string, Intl.Collator>();

/** Cached name collator of the locale with numeric digits, so `Lith A2` sorts before `Lith A10`. */
export function nameCollator(locale: string): Intl.Collator {
  let collator = collators.get(locale);
  if (!collator) {
    collator = new Intl.Collator(locale, { numeric: true });
    collators.set(locale, collator);
  }
  return collator;
}

/** Compares two names with the collator in the sort direction. */
export function compareNames(
  collator: Intl.Collator,
  one: string,
  other: string,
  descending: boolean,
): number {
  const order = collator.compare(one, other);
  return descending ? -order : order;
}

/** Initial direction of a newly picked sort key: descending, except ascending for `name`. */
export function firstWay(by: string): boolean {
  return by !== "name";
}
