import type { Good } from "@median/ipc";
import { useGoods, useSets, useTaxonomy } from "@/shared/data/catalog";
import { useFavourites } from "@/shared/data/favourites";
import { useHoldings } from "@/shared/data/holdings";
import { priceKey, useOwnOrders, usePrices, useQuotes, useSetPrices } from "@/shared/data/market";
import type { Marks } from "../model/facets";
import { kitsOf } from "../model/kits";
import { type HeldLine, heldLines, type InventoryTab, tabOf } from "../model/lines";

/** Inventory page data combined from the account, the catalog and the market cache. */
export function useInventory() {
  const holdings = useHoldings();
  const pieces = holdings.data?.pieces ?? [];
  const sets = useSets().data ?? [];
  const held = new Map<string, number>();
  for (const piece of pieces) held.set(piece.item, (held.get(piece.item) ?? 0) + piece.count);
  const touched = sets.filter((set) => set.members.some((member) => held.has(member)));
  const items = [
    ...new Set([...held.keys(), ...touched.flatMap((set) => (set.built ? [set.built] : []))]),
  ].sort();
  const goodsAnswer = useGoods(items);
  const goods = new Map((goodsAnswer.data ?? []).map((good: Good) => [good.item, good]));
  const taxonomy = useTaxonomy().data;
  const prices = usePrices().data;
  const setPrices = useSetPrices().data;
  const favourites = useFavourites().data;
  const own = (useOwnOrders().data ?? []).filter((order) => order.selling);
  const kitItems = [
    ...new Set(touched.flatMap((set) => [...set.members, ...(set.built ? [set.built] : [])])),
  ].sort();
  const kitGoods = new Map((useGoods(kitItems).data ?? []).map((good: Good) => [good.item, good]));
  const quotes = useQuotes(kitItems).data;

  const classes = new Map((taxonomy?.kinds ?? []).map((kind) => [kind.slug, kind.class]));
  const lines = heldLines({
    pieces,
    goods,
    classes,
    sets: touched,
    held,
    price: (item, variant) => prices?.get(priceKey(item, variant)),
    setPrice: (slug) => setPrices?.get(slug),
  });
  const members = new Set(touched.flatMap((set) => set.members));
  const byTab = new Map<InventoryTab, HeldLine[]>();
  for (const line of lines) {
    const tab = tabOf(line, members);
    if (!tab) continue;
    const list = byTab.get(tab);
    if (list) list.push(line);
    else byTab.set(tab, [line]);
  }
  const listingOf = (line: HeldLine) =>
    own.find((order) =>
      line.pair !== null
        ? order.slug === line.pair.slug && order.variant === line.pair.key
        : order.item === line.item,
    );
  const marks: Marks = {
    listed: (line) => listingOf(line) !== undefined,
    favourite: (line) => favourites?.has(line.item) ?? false,
  };

  return {
    /** Item keys of every held item and of every set a held part belongs to, sorted. */
    items,
    /** Whether the inventory and its catalog rows have loaded. */
    ready: holdings.data !== undefined && (items.length === 0 || goodsAnswer.data !== undefined),
    empty: pieces.length === 0,
    lines: (tab: InventoryTab) => byTab.get(tab) ?? [],
    marks,
    /** Catalog row of an item in `items`, such as an item copies are installed on. */
    good: (item: string) => goods.get(item),
    /** Every set the account holds a part of, with its missing parts. */
    kits: kitsOf(
      touched,
      held,
      (item) => kitGoods.get(item) ?? goods.get(item),
      (item) => quotes?.get(item) ?? null,
      (slug) => setPrices?.get(slug)?.worth ?? null,
    ),
    /** The account's open sell order for the line, if any. */
    listing: listingOf,
    /** The account's open sell order for an item, or for any order under the market slug. */
    listingFor: (item: string, slug: string | null) =>
      own.find((order) => order.item === item || (slug !== null && order.slug === slug)),
    /** Market cache price of one variant of an item. */
    price: (item: string, variant: string) => prices?.get(priceKey(item, variant)),
  };
}
