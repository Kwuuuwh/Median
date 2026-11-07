import type { Offer } from "@median/ipc";
import { useState } from "react";
import { ITEM_TILE_WIDTH } from "@/features/items/ui/ItemTile";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { EmptyState } from "@/shared/ui/empty-state";
import { Segmented } from "@/shared/ui/segmented";
import { Tab, TabList, TabPanel, Tabs } from "@/shared/ui/tabs";
import { TileGrid } from "@/shared/ui/tile-grid";
import { type ShelfTab, shelfTabs } from "../model/shelf";
import { OfferTile } from "./OfferTile";
import { ShelfHead } from "./ShelfHead";
import { useShelf } from "./useShelf";

/**
 * Baro's offers by tab: each offer's price and the copies the account holds, with a filter for
 * items the account lacks.
 */
export function BaroShelf({ offers }: { offers: readonly Offer[] }) {
  const t = useT();
  const locale = useLocale();
  const [search, setSearch] = useState("");
  const [open, setOpen] = useState<ShelfTab | null>(null);
  const [missing, setMissing] = useState(false);
  const shelf = useShelf(offers);
  if (offers.length === 0) return <EmptyState title={t("trader-empty")} />;

  const ducats = shelf.wallet("ducats");
  const every = shelfTabs({
    offers,
    goods: shelf.goods,
    kinds: shelf.kinds,
    pieces: shelf.pieces,
    quotes: shelf.quotes,
    search,
    locale,
  });
  const tabs = every
    .map((each) => ({
      ...each,
      lines: missing ? each.lines.filter((line) => line.held === 0 && !line.mastered) : each.lines,
    }))
    .filter((each) => each.lines.length > 0);
  const current = tabs.find((each) => each.tab === open)?.tab ?? tabs[0]?.tab;

  return (
    <div className="grid gap-3">
      <ShelfHead
        search={search}
        onSearch={setSearch}
        wallet={ducats === null ? null : { currency: "ducats", amount: ducats }}
      >
        <Segmented<"all" | "missing">
          label={t("shelf-missing-only")}
          className="h-8"
          value={missing ? "missing" : "all"}
          onChange={(value) => setMissing(value === "missing")}
          segments={[
            { value: "all", label: t("shelf-all") },
            { value: "missing", label: t("shelf-missing-only") },
          ]}
        />
      </ShelfHead>
      {!current ? (
        <EmptyState title={t("shelf-nothing")} />
      ) : (
        <Tabs value={current} onValueChange={(value) => setOpen(value as ShelfTab)}>
          <TabList>
            {tabs.map((each) => (
              <Tab key={each.tab} value={each.tab} count={each.lines.length}>
                {t(`shelf-tab-${each.tab}`)}
              </Tab>
            ))}
          </TabList>
          {tabs.map((each) => (
            <TabPanel key={each.tab} value={each.tab} className="pt-3">
              <TileGrid least={ITEM_TILE_WIDTH}>
                {each.lines.map((line) => (
                  <OfferTile key={line.offer.item} line={line} wallet={ducats} />
                ))}
              </TileGrid>
            </TabPanel>
          ))}
        </Tabs>
      )}
    </div>
  );
}
