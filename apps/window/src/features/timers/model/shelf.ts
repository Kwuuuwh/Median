import type { Currency, Good, Offer, Piece } from "@median/ipc";
import { nameFromKey } from "@/features/items/model/name";
import { matcher } from "@/shared/lib/search";
import { nameCollator } from "@/shared/lib/sort";

/** A tab of a vendor's offer list. */
export type ShelfTab =
  | "warframes"
  | "weapons"
  | "mods"
  | "relics"
  | "cosmetics"
  | "other"
  | "bundles"
  | "regal";

/** Tabs in display order: gear first, offers paid in Regal Aya (bought with real money) last. */
const SHELF_TABS: readonly ShelfTab[] = [
  "warframes",
  "weapons",
  "mods",
  "relics",
  "cosmetics",
  "other",
  "bundles",
  "regal",
];

/** Tab of each catalog class; an unlisted class goes under `other`. */
const CLASS_TAB: Record<string, ShelfTab> = {
  warframe: "warframes",
  primary: "weapons",
  secondary: "weapons",
  melee: "weapons",
  "archwing-weapon": "weapons",
  "companion-weapon": "weapons",
  exalted: "weapons",
  modular: "weapons",
  mod: "mods",
  arcane: "mods",
  relic: "relics",
  cosmetic: "cosmetics",
};

/** One offer as the vendor list shows it. */
export type ShelfLine = {
  offer: Offer;
  name: string;
  image: string | null;
  /** Whether the item is in the catalog; an unknown one is named from its key. */
  known: boolean;
  /** Catalog kind slug, when the item is in the catalog. */
  kind: string | null;
  /** Copies of the item the account holds. */
  held: number;
  mastered: boolean;
  /** Market price of the item in platinum. */
  platinum: number | null;
};

/** Inputs the vendor list is built from. */
export type ShelfSources = {
  offers: readonly Offer[];
  goods: ReadonlyMap<string, Good>;
  /** Class of each catalog kind. */
  kinds: ReadonlyMap<string, string>;
  pieces: readonly Piece[];
  /** Market price of each item that has one, in platinum. */
  quotes: ReadonlyMap<string, number>;
  search: string;
  locale: string;
};

/** Currency of the offer's first price; null for an offer without prices. */
function paidIn(offer: Offer): Currency | null {
  return offer.prices[0]?.currency ?? null;
}

/** Offer tab: `regal` for Regal Aya, `bundles` for items the catalog lacks, else by class. */
function tabOf(offer: Offer, good: Good | undefined, kinds: ReadonlyMap<string, string>): ShelfTab {
  if (paidIn(offer) === "regalAya") return "regal";
  if (!good) return "bundles";
  return CLASS_TAB[kinds.get(good.kind) ?? ""] ?? "other";
}

/**
 * Offers matching the search, grouped by tab in display order; each tab sorted by market price,
 * highest first and unpriced last, then by name. Empty tabs are omitted.
 */
export function shelfTabs(sources: ShelfSources): { tab: ShelfTab; lines: ShelfLine[] }[] {
  const held = new Map<string, number>();
  for (const piece of sources.pieces)
    held.set(piece.item, (held.get(piece.item) ?? 0) + piece.count);
  const matches = matcher(sources.search);
  const collator = nameCollator(sources.locale);

  const tabs = new Map<ShelfTab, ShelfLine[]>();
  for (const offer of sources.offers) {
    const good = sources.goods.get(offer.item);
    const name = good?.name ?? nameFromKey(offer.item);
    if (!matches(name)) continue;
    const platinum = sources.quotes.get(offer.item) ?? null;
    const tab = tabOf(offer, good, sources.kinds);
    tabs.set(tab, [
      ...(tabs.get(tab) ?? []),
      {
        offer,
        name,
        image: good?.image ?? null,
        known: good !== undefined,
        kind: good?.kind ?? null,
        held: held.get(offer.item) ?? 0,
        mastered: good?.mastered === true,
        platinum,
      },
    ]);
  }

  return SHELF_TABS.flatMap((tab) => {
    const lines = tabs.get(tab);
    if (!lines) return [];
    lines.sort(
      (a, b) =>
        (a.platinum === null ? 1 : 0) - (b.platinum === null ? 1 : 0) ||
        (b.platinum ?? 0) - (a.platinum ?? 0) ||
        collator.compare(a.name, b.name),
    );
    return [{ tab, lines }];
  });
}
