import { useState } from "react";
import { CARD_HEIGHT, CARD_WIDTH } from "@/features/items/ui/ItemCard";
import { useLocale } from "@/shared/data/app-state";
import { useItemSearch } from "@/shared/data/search";
import { useShown } from "@/shared/data/shown";
import { useT } from "@/shared/i18n";
import { nameCollator, type Sorting } from "@/shared/lib/sort";
import { useKept } from "@/shared/lib/use-kept";
import { EmptyState } from "@/shared/ui/empty-state";
import { FacetMenu } from "@/shared/ui/facet-menu";
import { Money } from "@/shared/ui/money";
import { wholeNumber } from "@/shared/ui/number-field";
import { SearchField } from "@/shared/ui/search-field";
import { Segmented } from "@/shared/ui/segmented";
import { SortSelect } from "@/shared/ui/sort-select";
import { Tab, TabList, Tabs } from "@/shared/ui/tabs";
import { TileGrid } from "@/shared/ui/tile-grid";
import { VirtualGrid } from "@/shared/ui/virtual-grid";
import { type Facets, passes, TAB_FACETS, TAB_ORDERS } from "../model/facets";
import { type KitOrder, kitWay, sortKits } from "../model/kits";
import {
  type HeldLine,
  type InventoryTab,
  inPriceRange,
  inRange,
  type LineOrder,
  shortNames,
  sortLines,
  totals,
} from "../model/lines";
import { HaulBadge } from "./HaulBadge";
import { HeldCard } from "./HeldCard";
import { KitTile } from "./KitTile";
import { useInventory } from "./useInventory";

/** The tabs, in display order. */
const TABS: readonly InventoryTab[] = ["parts", "mods", "arcanes", "sets", "other"];

/** Filters and sort order of one tab. */
type View = {
  order: Sorting<LineOrder>;
  facets: Facets;
  /** Price bounds of one copy, as typed. */
  from: string;
  to: string;
  /** The arcane collection shown, `ALL` or `UNCOLLECTED`; undefined counts as `ALL`. */
  collection?: string;
};

/** Collection choices for every arcane, and for arcanes no collection rolls. */
const ALL = "?all";
const UNCOLLECTED = "?none";

const FRESH: View = {
  order: { by: "platinum", descending: true },
  facets: new Map(),
  from: "",
  to: "",
};

/** Sort keys of the sets tab. */
const KIT_ORDERS: readonly KitOrder[] = ["ready", "platinum", "missing"];

/** Highest allowed price bound, in platinum. */
const MOST_PRICE = 100_000;

/** The inventory by tab, with filters and sort order per tab; each item opens its card. */
export function InventoryPage() {
  const t = useT();
  const locale = useLocale();
  const inventory = useInventory();
  const [open, setOpen] = useKept<InventoryTab>("inventory.tab", "parts");
  const search = useItemSearch("inventory.search", inventory.items);
  const [scroller, setScroller] = useState<HTMLDivElement | null>(null);
  const [views, setViews] = useKept<Partial<Record<InventoryTab, View>>>("inventory.views", {});

  const searched = (tab: InventoryTab) =>
    inventory.lines(tab).filter((line) => search.matches(line.item, line.name));
  const [kitOrder, setKitOrder] = useKept<Sorting<KitOrder>>("inventory.kits", {
    by: "ready",
    descending: true,
  });
  const kits = sortKits(
    inventory.kits.filter(
      (kit) =>
        search.matches(kit.item, kit.name) &&
        inRange(
          kit.price,
          wholeNumber((views.sets ?? FRESH).from, 0, MOST_PRICE),
          wholeNumber((views.sets ?? FRESH).to, 0, MOST_PRICE),
        ),
    ),
    kitOrder,
  );
  const tabs = TABS.filter((tab) =>
    tab === "sets" ? inventory.kits.length > 0 : inventory.lines(tab).length > 0,
  );
  const current = tabs.includes(open) ? open : (tabs[0] ?? open);
  const view = views[current] ?? FRESH;
  const setView = (change: Partial<View>) =>
    setViews({ ...views, [current]: { ...view, ...change } });

  const collection = current === "arcanes" ? (view.collection ?? ALL) : ALL;
  const lines = sortLines(
    searched(current).filter(
      (line) =>
        (collection === ALL ||
          line.collection === (collection === UNCOLLECTED ? null : collection)) &&
        passes(line, view.facets, inventory.marks) &&
        inPriceRange(
          line,
          wholeNumber(view.from, 0, MOST_PRICE),
          wholeNumber(view.to, 0, MOST_PRICE),
        ),
    ),
    view.order,
    locale,
  );
  useShown(lines.flatMap((line) => (line.pair ? [line.pair] : [])));

  if (!inventory.ready || inventory.empty) return <EmptyState title={t("inventory-waiting")} />;

  const sum = totals(lines, view.facets.get("spare") === true);
  const arcanes = inventory.lines("arcanes");
  const collections = [
    ...new Set(arcanes.flatMap((line) => (line.collection ? [line.collection] : []))),
  ].sort(nameCollator(locale).compare);
  const labels = shortNames(collections);

  return (
    <Tabs
      value={current}
      onValueChange={(value) => setOpen(value as InventoryTab)}
      className="relative flex min-w-0 flex-1 flex-col"
    >
      <div className="flex shrink-0 flex-col border-line border-b px-6 pt-3">
        <TabList className="shrink-0 border-b-0">
          {tabs.map((tab) => (
            <Tab key={tab} value={tab} count={tab === "sets" ? kits.length : searched(tab).length}>
              {t("inventory-tab", { tab })}
            </Tab>
          ))}
        </TabList>
        {current === "arcanes" && collections.length > 0 && (
          <Segmented
            label={t("inventory-collections")}
            className="self-start"
            value={collection}
            onChange={(picked) => setView({ collection: picked })}
            segments={[
              { value: ALL, label: t("inventory-collections") },
              ...collections.map((name) => ({ value: name, label: labels.get(name) ?? name })),
              ...(arcanes.some((line) => line.collection === null)
                ? [{ value: UNCOLLECTED, label: t("inventory-uncollected") }]
                : []),
            ]}
          />
        )}
        <div className="flex flex-wrap items-center gap-2 py-2.5">
          <SearchField
            {...search.field}
            placeholder={t("inventory-search")}
            className="w-56 min-w-36 shrink"
          />
          <FacetMenu
            facets={current === "sets" ? [] : TAB_FACETS[current]}
            values={view.facets}
            onChange={(values) => setView({ facets: values })}
            ranges={[
              {
                label: t("inventory-price"),
                from: view.from,
                to: view.to,
                invalidFrom: view.from !== "" && wholeNumber(view.from, 0, MOST_PRICE) === null,
                invalidTo: view.to !== "" && wholeNumber(view.to, 0, MOST_PRICE) === null,
                onChange: (from, to) => setView({ from, to }),
              },
            ]}
            onClear={() => setView({ facets: new Map(), from: "", to: "" })}
            className="w-44"
          />
          {current === "sets" ? (
            <SortSelect<KitOrder>
              label={t("shelf-sort")}
              className="w-44"
              value={kitOrder}
              onChange={setKitOrder}
              wayOf={kitWay}
              choices={KIT_ORDERS.map((order) => ({
                value: order,
                label: t("kit-order", { order }),
              }))}
            />
          ) : (
            <SortSelect<LineOrder>
              label={t("shelf-sort")}
              className="w-36"
              value={view.order}
              onChange={(order) => setView({ order })}
              choices={TAB_ORDERS[current].map((order) => ({
                value: order,
                label: t("inventory-order", { order }),
              }))}
            />
          )}
        </div>
      </div>
      <div ref={setScroller} className="relative min-h-0 flex-1 overflow-y-auto px-6 pt-4 pb-16">
        {current === "sets" ? (
          kits.length === 0 ? (
            <EmptyState title={t("inventory-nothing-found")} />
          ) : (
            <TileGrid least={336} className="items-start">
              {kits.map((kit) => (
                <KitTile key={kit.slug} kit={kit} listingFor={inventory.listingFor} />
              ))}
            </TileGrid>
          )
        ) : lines.length === 0 ? (
          <EmptyState title={t("inventory-nothing-found")} />
        ) : (
          <VirtualGrid
            key={current}
            items={lines}
            scroller={scroller}
            least={CARD_WIDTH}
            estimate={CARD_HEIGHT}
            keyOf={(line) => `${line.group}\u0000${line.item}\u0000${line.variant}`}
          >
            {(line: HeldLine) => (
              <HeldCard line={line} good={inventory.good} listing={inventory.listing(line)} />
            )}
          </VirtualGrid>
        )}
      </div>
      <div className="pointer-events-none absolute right-6 bottom-4 inline-flex items-center gap-3 rounded-full border border-line bg-raised/90 px-3.5 py-1.5 shadow-black/40 shadow-lg backdrop-blur-sm">
        <Money
          currency="platinum"
          amount={sum.platinum}
          className="font-medium text-body text-ink"
        />
        {sum.ducats > 0 && (
          <Money currency="ducats" amount={sum.ducats} className="text-body text-soft" />
        )}
      </div>
      <HaulBadge />
    </Tabs>
  );
}
