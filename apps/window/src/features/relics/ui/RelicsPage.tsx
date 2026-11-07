import { keepPreviousData } from "@tanstack/react-query";
import { HaulBadge } from "@/features/inventory/ui/HaulBadge";
import { SQUAD, useRelics } from "@/shared/data/catalog";
import { useT } from "@/shared/i18n";
import { matcher } from "@/shared/lib/search";
import type { Sorting } from "@/shared/lib/sort";
import { useKept } from "@/shared/lib/use-kept";
import { EmptyState } from "@/shared/ui/empty-state";
import { SearchField } from "@/shared/ui/search-field";
import { Segmented } from "@/shared/ui/segmented";
import { SortSelect } from "@/shared/ui/sort-select";
import { TileGrid } from "@/shared/ui/tile-grid";
import { erasOf, type RelicOrder, relicRows } from "../model/rows";
import { RelicTile } from "./RelicTile";

const ORDERS = [
  ["platinum", "relics-by-platinum"],
  ["upgrade", "relics-by-upgrade"],
  ["ducats", "relics-by-ducats"],
  ["count", "relics-by-count"],
] as const;

/** Relic eras in the game's unlock order. */
const ERAS = ["lith", "meso", "neo", "axi", "requiem", "vanguard"] as const;

const ANY = "any";

/** Every relic the account holds: average run value of each held refinement, and its rewards. */
export function RelicsPage() {
  const t = useT();
  const [squad, setSquad] = useKept("relics.squad", SQUAD);
  const relics = useRelics(squad, keepPreviousData).data;
  const [search, setSearch] = useKept("relics.search", "");
  const [era, setEra] = useKept<string>("relics.era", ANY);
  const [order, setOrder] = useKept<Sorting<RelicOrder>>("relics.order", {
    by: "platinum",
    descending: true,
  });
  if (!relics) return <EmptyState title={t("relics-title")} />;

  const shown = relicRows(relics, era === ANY ? null : era, order, matcher(search));
  const eras = erasOf(relics, ERAS);

  return (
    <div className="relative flex min-w-0 flex-1 flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 border-line border-b px-6 py-3">
        {eras.length > 1 && (
          <Segmented<string>
            label={t("relics-era")}
            value={era}
            onChange={setEra}
            segments={[
              { value: ANY, label: t("relics-any-era") },
              ...eras.map((each) => ({ value: each, label: t("era-name", { era: each }) })),
            ]}
          />
        )}
        <span className="flex-1" />
        <SearchField value={search} onChange={setSearch} placeholder={t("relics-search")} />
        <Segmented<string>
          label={t("relics-squad")}
          value={String(squad)}
          onChange={(value) => setSquad(Number(value))}
          segments={["1", "2", "3", "4"].map((each) => ({ value: each, label: each }))}
        />
        <SortSelect<RelicOrder>
          label={t("relics-sort")}
          className="w-40"
          value={order}
          onChange={setOrder}
          choices={ORDERS.map(([value, label]) => ({ value, label: t(label) }))}
        />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-4 pb-16">
        {shown.length === 0 ? (
          <EmptyState title={t("relics-none")} />
        ) : (
          <TileGrid least={352} className="items-start">
            {shown.map((relic) => (
              <RelicTile key={relic.base} relic={relic} />
            ))}
          </TileGrid>
        )}
      </div>
      <HaulBadge />
    </div>
  );
}
