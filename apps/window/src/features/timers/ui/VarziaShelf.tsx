import type { Offer, Relic } from "@median/ipc";
import { useEffect, useState } from "react";
import { ITEM_TILE_WIDTH } from "@/features/items/ui/ItemTile";
import { RelicCard } from "@/features/items/ui/RelicCard";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { matcher } from "@/shared/lib/search";
import { compareMeasures, type Sorting } from "@/shared/lib/sort";
import { EmptyState } from "@/shared/ui/empty-state";
import { Money } from "@/shared/ui/money";
import { SortSelect } from "@/shared/ui/sort-select";
import { Tab, TabList, TabPanel, Tabs } from "@/shared/ui/tabs";
import { TileGrid } from "@/shared/ui/tile-grid";
import { rotationOf, wantedParts } from "../model/rotation";
import { type ShelfLine, shelfTabs } from "../model/shelf";
import { OfferTile } from "./OfferTile";
import { PrimeSetCard } from "./PrimeSetCard";
import { ShelfHead } from "./ShelfHead";
import { useShelf } from "./useShelf";

type VarziaTab = "primes" | "relics" | "cosmetics" | "regal";
type RelicOrder = "payback" | "top" | "held";

/** Group key of items unknown to the catalog. */
const UNKNOWN = "?unknown";

/** How long a relic stays highlighted after it is opened from a set card, in milliseconds. */
const FOCUS_MS = 1600;

/**
 * Varzia's offers: the prime sets in rotation part by part, the relics that drop them, and the
 * other offers grouped by kind.
 */
export function VarziaShelf({ offers }: { offers: readonly Offer[] }) {
  const t = useT();
  const locale = useLocale();
  const [search, setSearch] = useState("");
  const [open, setOpen] = useState<VarziaTab | null>(null);
  const [focus, setFocus] = useState<string | null>(null);
  const [order, setOrder] = useState<Sorting<RelicOrder>>({ by: "payback", descending: true });
  const shelf = useShelf(offers);

  useEffect(() => {
    if (!focus) return;
    document
      .getElementById(`relic-${focus}`)
      ?.scrollIntoView({ block: "center", behavior: "smooth" });
    const timer = setTimeout(() => setFocus(null), FOCUS_MS);
    return () => clearTimeout(timer);
  }, [focus]);

  if (offers.length === 0) return <EmptyState title={t("trader-empty")} />;

  const aya = shelf.wallet("aya");
  const matches = matcher(search);
  const byTab = new Map(
    shelfTabs({
      offers,
      goods: shelf.goods,
      kinds: shelf.kinds,
      pieces: shelf.pieces,
      quotes: shelf.quotes,
      search,
      locale,
    }).map((each) => [each.tab, each.lines]),
  );
  const relics = [
    ...new Map([...shelf.relics.values()].map((relic) => [relic.base, relic])).values(),
  ];
  const rotation = rotationOf({
    offers,
    sets: shelf.sets,
    relics,
    goods: shelf.goods,
    held: shelf.held,
    quotes: shelf.quotes,
    order: shelf.order,
    locale,
  });
  const wanted = wantedParts(rotation);
  const sets = rotation.filter((set) => matches(set.name));
  const inRotation = new Set(rotation.map((set) => set.built));

  const shelved = (byTab.get("relics") ?? []).flatMap((line) => {
    const relic = shelf.relics.get(line.offer.item);
    return relic ? [{ line, relic }] : [];
  });
  const payOf = (relic: Relic) => relic.steps[0]?.platinum ?? 0;
  const topOf = (relic: Relic) => Math.max(0, ...relic.holds.map((slot) => slot.platinum ?? 0));
  const heldOf = (relic: Relic) => relic.steps.reduce((sum, step) => sum + step.count, 0);
  const measure = order.by === "top" ? topOf : order.by === "held" ? heldOf : payOf;
  shelved.sort(
    (a, b) =>
      compareMeasures(measure(a.relic), measure(b.relic), order.descending) ||
      payOf(b.relic) - payOf(a.relic),
  );
  const cosmetics: ShelfLine[] = [
    ...(byTab.get("cosmetics") ?? []),
    ...(byTab.get("other") ?? []),
    ...(byTab.get("bundles") ?? []),
  ];
  const regal = (byTab.get("regal") ?? []).filter((line) => !inRotation.has(line.offer.item));

  const tabs: { tab: VarziaTab; count: number }[] = [
    { tab: "primes" as const, count: sets.length },
    { tab: "relics" as const, count: shelved.length },
    { tab: "cosmetics" as const, count: cosmetics.length },
    { tab: "regal" as const, count: regal.length },
  ].filter((each) => each.count > 0);
  const current = tabs.find((each) => each.tab === open)?.tab ?? tabs[0]?.tab;

  /** Tiles under one heading per kind, kinds in catalog order. */
  const grouped = (lines: readonly ShelfLine[], wallet: number | null) => {
    const bySlug = new Map<string, ShelfLine[]>();
    for (const line of lines) {
      const slug = line.known ? (line.kind ?? "") : UNKNOWN;
      bySlug.set(slug, [...(bySlug.get(slug) ?? []), line]);
    }
    return [...bySlug]
      .map(([slug, some]) => ({
        slug,
        name:
          slug === UNKNOWN ? t("shelf-unknown") : (shelf.kind(slug)?.name ?? t("shelf-tab-other")),
        ord:
          slug === UNKNOWN
            ? Number.POSITIVE_INFINITY
            : (shelf.kind(slug)?.ord ?? Number.MAX_SAFE_INTEGER),
        lines: some,
      }))
      .sort((a, b) => a.ord - b.ord)
      .map((group) => (
        <section key={group.slug} className="grid gap-2">
          <h3 className="flex items-baseline gap-1.5 text-muted text-small">
            {group.name}
            <span className="text-dim tabular-nums">{group.lines.length}</span>
          </h3>
          <TileGrid least={ITEM_TILE_WIDTH}>
            {group.lines.map((line) => (
              <OfferTile key={line.offer.item} line={line} wallet={wallet} />
            ))}
          </TileGrid>
        </section>
      ));
  };

  return (
    <div className="grid gap-3">
      <ShelfHead
        search={search}
        onSearch={setSearch}
        wallet={aya === null ? null : { currency: "aya", amount: aya }}
      />
      {!current ? (
        <EmptyState title={t("shelf-nothing")} />
      ) : (
        <Tabs value={current} onValueChange={(value) => setOpen(value as VarziaTab)}>
          <TabList>
            {tabs.map((each) => (
              <Tab key={each.tab} value={each.tab} count={each.count}>
                {t(each.tab === "primes" ? "shelf-tab-primes" : `shelf-tab-${each.tab}`)}
              </Tab>
            ))}
          </TabList>
          <TabPanel value="primes" className="columns-[24rem] gap-3 pt-3">
            {sets.map((set) => (
              <PrimeSetCard
                key={set.built}
                set={set}
                onRelic={(base) => {
                  setOpen("relics");
                  setFocus(base);
                }}
              />
            ))}
          </TabPanel>
          <TabPanel value="relics" className="grid gap-3 pt-3">
            <SortSelect<RelicOrder>
              label={t("shelf-sort")}
              className="w-44"
              value={order}
              onChange={setOrder}
              choices={[
                { value: "payback", label: t("shelf-sort-payback") },
                { value: "top", label: t("shelf-sort-top") },
                { value: "held", label: t("shelf-sort-held") },
              ]}
            />
            <TileGrid least={320}>
              {shelved.map(({ line, relic }) => (
                <RelicCard
                  key={relic.base}
                  relic={relic}
                  held={relic.steps[0]?.count ?? 0}
                  meta={
                    <Money
                      currency="aya"
                      amount={line.offer.prices[0]?.amount ?? 0}
                      className="font-medium text-ink"
                    />
                  }
                  wanted={wanted}
                  focused={focus === relic.base}
                />
              ))}
            </TileGrid>
          </TabPanel>
          <TabPanel value="cosmetics" className="grid gap-5 pt-3">
            {grouped(cosmetics, aya)}
          </TabPanel>
          <TabPanel value="regal" className="grid gap-5 pt-3">
            {grouped(regal, null)}
          </TabPanel>
        </Tabs>
      )}
    </div>
  );
}
