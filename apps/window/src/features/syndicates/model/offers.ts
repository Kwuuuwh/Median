import type { StandingOffer, Syndicate } from "@median/ipc";
import { compareMeasures, type Sorting } from "@/shared/lib/sort";

/** Sort keys of a syndicate's offers. */
export type OfferOrder = "platinum" | "cost";

/** A yes/no filter for syndicate offers. */
export type OfferFacet = "held" | "listed" | "tradable" | "opened" | "affordable";

/** Filters the offer list offers, in display order. */
export const OFFER_FACETS: readonly OfferFacet[] = [
  "held",
  "listed",
  "tradable",
  "opened",
  "affordable",
];

/** Inclusive lower and upper bound of a value; null leaves that side open. */
export type Bounds = { least: number | null; most: number | null };

/** Filters and value ranges of an offer list. */
export type OfferFilter = {
  facets: ReadonlyMap<OfferFacet, boolean>;
  platinum: Bounds;
  volume: Bounds;
};

export const OPEN: Bounds = { least: null, most: null };

/** Whether the account's syndicate rank unlocks the offer; true for an offer without a rank. */
export function isOpened(offer: StandingOffer, syndicate: Syndicate): boolean {
  return offer.rank === null || offer.rank <= syndicate.title;
}

/** Whether the account has enough syndicate standing to buy the offer. */
export function isAffordable(offer: StandingOffer, syndicate: Syndicate): boolean {
  return offer.cost <= syndicate.standing;
}

/** An offer's value for a filter, given the syndicate rank, standing and listed items. */
function answer(
  facet: OfferFacet,
  offer: StandingOffer,
  syndicate: Syndicate,
  listed: ReadonlySet<string>,
): boolean {
  switch (facet) {
    case "held":
      return offer.owned > 0;
    case "listed":
      return listed.has(offer.item);
    case "tradable":
      return offer.tradable;
    case "opened":
      return isOpened(offer, syndicate);
    case "affordable":
      return isAffordable(offer, syndicate);
  }
}

/** Whether a value lies within the bounds; a null value fails any set bound. */
function within(value: number | null, bounds: Bounds): boolean {
  if (bounds.least === null && bounds.most === null) return true;
  if (value === null) return false;
  return (
    (bounds.least === null || value >= bounds.least) &&
    (bounds.most === null || value <= bounds.most)
  );
}

/** Initial direction of a newly picked key: highest platinum first, lowest standing cost first. */
export function offerWay(by: OfferOrder): boolean {
  return by === "platinum";
}

/** Sort value of an offer for a key; null for an unknown price. */
function measure(by: OfferOrder, offer: StandingOffer): number | null {
  return by === "platinum" ? offer.platinum : offer.cost;
}

/**
 * Offers that pass the search and filter, sorted by the chosen key and direction, ties by name;
 * `listed` holds the item keys the account has sell orders for.
 */
export function shownOffers(
  syndicate: Syndicate,
  filter: OfferFilter,
  sorting: Sorting<OfferOrder>,
  matches: (name: string) => boolean,
  listed: ReadonlySet<string>,
): StandingOffer[] {
  return syndicate.offers
    .filter((offer) => matches(offer.name))
    .filter((offer) =>
      [...filter.facets].every(
        ([facet, wanted]) => answer(facet, offer, syndicate, listed) === wanted,
      ),
    )
    .filter((offer) => within(offer.platinum, filter.platinum))
    .filter((offer) => within(offer.perDay, filter.volume))
    .sort(
      (a, b) =>
        compareMeasures(measure(sorting.by, a), measure(sorting.by, b), sorting.descending) ||
        a.name.localeCompare(b.name),
    );
}
