import type { OwnOrder } from "@median/ipc";
import { compareMeasures, compareNames, nameCollator, type Sorting } from "@/shared/lib/sort";

/** How the account's order compares with the best competing order from reachable players. */
export type Standing = "best" | "beaten" | "unknown";

/** Sort keys of the order list. */
export type BookOrder = "name" | "price" | "gap" | "quantity" | "worth";

/** A yes/no filter for the account's orders. */
export type BookFacet = "beaten" | "visible" | "agent";

/** Filters the order list offers, in display order. */
export const BOOK_FACETS: readonly BookFacet[] = ["beaten", "visible", "agent"];

/**
 * Platinum by which the order's price is worse than the best competing reachable price on its
 * side; negative when it is better, null when there is no competing order.
 */
export function gapOf(order: OwnOrder): number | null {
  if (order.reference === null) return null;
  return order.selling ? order.platinum - order.reference : order.reference - order.platinum;
}

/** Whether the order has the best price on its side (`best`), is outbid (`beaten`), or unknown. */
export function standingOf(order: OwnOrder): Standing {
  const gap = gapOf(order);
  if (gap === null) return "unknown";
  return gap > 0 ? "beaten" : "best";
}

/** Total platinum of the order: price times quantity. */
export function worthOf(order: OwnOrder): number {
  return order.platinum * order.quantity;
}

/** The order's value for a filter. */
export function answer(facet: BookFacet, order: OwnOrder): boolean {
  if (facet === "beaten") return standingOf(order) === "beaten";
  if (facet === "visible") return order.visible;
  return order.byAgent;
}

/** Orders sorted by the chosen key and direction; unknown values last, ties by name. */
export function sortBook(
  orders: readonly OwnOrder[],
  sorting: Sorting<BookOrder>,
  locale: string,
): OwnOrder[] {
  const { by, descending } = sorting;
  const collator = nameCollator(locale);
  const measure = (order: OwnOrder): number | null =>
    by === "price"
      ? order.platinum
      : by === "gap"
        ? gapOf(order)
        : by === "quantity"
          ? order.quantity
          : worthOf(order);
  return [...orders].sort((a, b) =>
    by === "name"
      ? compareNames(collator, a.name, b.name, descending)
      : compareMeasures(measure(a), measure(b), descending) || collator.compare(a.name, b.name),
  );
}
