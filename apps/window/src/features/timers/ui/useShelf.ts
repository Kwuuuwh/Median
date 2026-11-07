import type { Good, Offer, Relic } from "@median/ipc";
import { useGoods, useRelicsOf, useSets, useTaxonomy } from "@/shared/data/catalog";
import { useHoldings } from "@/shared/data/holdings";
import { usePairs, useQuotes } from "@/shared/data/market";
import { useShown } from "@/shared/data/shown";

/** Inventory item key of each currency the account holds as an item. */
const WALLET: Partial<Record<string, string>> = {
  ducats: "/Lotus/Types/Items/MiscItems/PrimeBucks",
  aya: "/Lotus/Types/Items/MiscItems/SchismKey",
};

/**
 * Data of a vendor's offer list from the catalog, the account and the market; marks the offered
 * items as shown so their prices refresh first.
 */
export function useShelf(offers: readonly Offer[]) {
  const items = [...new Set(offers.map((offer) => offer.item))].sort();
  const sets = useSets().data ?? [];
  const offered = new Set(items);
  const members = sets
    .filter((set) => set.built !== null && offered.has(set.built))
    .flatMap((set) => set.members)
    .sort();
  const goods = new Map(
    [...(useGoods(items).data ?? []), ...(useGoods(members).data ?? [])]
      .filter((good: Good) => good.known)
      .map((good) => [good.item, good]),
  );
  const taxonomy = useTaxonomy().data;
  const pieces = useHoldings().data?.pieces ?? [];
  const relics = useRelicsOf(items).data;
  const quotes = useQuotes([...items, ...members]).data ?? new Map<string, number>();
  const pairs = usePairs([...items, ...members]).data;
  useShown(pairs ? [...pairs.values()] : []);

  const held = new Map<string, number>();
  for (const piece of pieces) held.set(piece.item, (held.get(piece.item) ?? 0) + piece.count);
  const kinds = new Map((taxonomy?.kinds ?? []).map((kind) => [kind.slug, kind.class]));
  const kindRows = new Map((taxonomy?.kinds ?? []).map((kind) => [kind.slug, kind]));
  const classOrder = new Map((taxonomy?.classes ?? []).map((each) => [each.slug, each.ord]));

  return {
    goods,
    sets,
    kinds,
    pieces,
    held,
    quotes,
    relics: relics ?? new Map<string, Relic>(),
    /** Position of a kind's class in the catalog order; unknown classes sort last. */
    order: (kind: string) => classOrder.get(kinds.get(kind) ?? "") ?? Number.MAX_SAFE_INTEGER,
    /** A kind's name and position in the catalog order. */
    kind: (slug: string) => kindRows.get(slug),
    /** The account's amount of a currency held as an item; null for other currencies. */
    wallet: (currency: string) => {
      const item = WALLET[currency];
      return item ? (held.get(item) ?? 0) : null;
    },
  };
}
