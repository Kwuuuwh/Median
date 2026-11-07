import type { MasteryRank } from "@median/ipc";
import { useState } from "react";
import { useItemSearch } from "@/shared/data/search";
import { useT } from "@/shared/i18n";
import { useKept } from "@/shared/lib/use-kept";
import { EmptyState } from "@/shared/ui/empty-state";
import { FacetMenu } from "@/shared/ui/facet-menu";
import { SearchField } from "@/shared/ui/search-field";
import { Segmented } from "@/shared/ui/segmented";
import { VirtualGrid } from "@/shared/ui/virtual-grid";
import {
  MASTERY_FACETS,
  type MasteryFacet,
  SHELVES,
  type Shelf,
  shelfCounts,
  shownItems,
} from "../model/items";
import { MasteryTile, TILE_HEIGHT, TILE_WIDTH } from "./MasteryTile";

/** Category choice that shows every category. */
const ALL = "all";

/** A category's name with the number of items still to master in it. */
function ShelfLabel({ name, count }: { name: string; count: number }) {
  return (
    <>
      {name}
      <span className="text-muted tabular-nums">{count}</span>
    </>
  );
}

/**
 * Every item that gives mastery, sorted by name and filtered by category, search and yes/no
 * filters; each tile can add its item to the plan.
 */
export function ItemsView({
  standing,
  planned,
  onPlan,
  onPreview,
}: {
  standing: MasteryRank;
  planned: ReadonlySet<string>;
  onPlan: (item: string, on: boolean) => void;
  onPreview: (points: number) => void;
}) {
  const t = useT();
  const [scroller, setScroller] = useState<HTMLDivElement | null>(null);
  const [shelf, setShelf] = useKept<Shelf | typeof ALL>("mastery.shelf", ALL);
  const search = useItemSearch(
    "mastery.search",
    standing.masterable.map((item) => item.item),
  );
  const [facets, setFacets] = useKept<ReadonlyMap<MasteryFacet, boolean>>(
    "mastery.facets",
    new Map(),
  );
  const rank = standing.rank ?? 0;
  const counts = shelfCounts(standing.masterable);
  const shown = shownItems(standing.masterable, rank, {
    shelf: shelf === ALL ? null : shelf,
    search: (item) => search.matches(item.item, item.name),
    facets,
    planned,
  });
  const left = [...counts.values()].reduce((sum, count) => sum + count, 0);

  return (
    <>
      <div className="flex shrink-0 flex-wrap items-center gap-2 px-6 pt-3">
        <Segmented<Shelf | typeof ALL>
          label={t("shelf-group-label")}
          value={shelf}
          onChange={setShelf}
          segments={[
            { value: ALL, label: <ShelfLabel name={t("mastery-all-shelves")} count={left} /> },
            ...SHELVES.map((each) => ({
              value: each,
              label: (
                <ShelfLabel
                  name={t("mastery-shelf", { shelf: each })}
                  count={counts.get(each) ?? 0}
                />
              ),
            })),
          ]}
        />
        <span className="flex-1" />
        <SearchField
          {...search.field}
          placeholder={t("mastery-search")}
          className="w-56 min-w-36 shrink"
        />
        <FacetMenu facets={MASTERY_FACETS} values={facets} onChange={setFacets} className="w-44" />
      </div>
      <div ref={setScroller} className="relative min-h-0 flex-1 overflow-y-auto px-6 pt-3 pb-16">
        {shown.length === 0 ? (
          <EmptyState title={t("mastery-none")} />
        ) : (
          <VirtualGrid
            items={shown}
            scroller={scroller}
            least={TILE_WIDTH}
            estimate={TILE_HEIGHT}
            keyOf={(item) => item.item}
          >
            {(item) => (
              <MasteryTile
                item={item}
                rank={rank}
                planned={planned.has(item.item)}
                onPlan={(on) => onPlan(item.item, on)}
                onPreview={onPreview}
              />
            )}
          </VirtualGrid>
        )}
      </div>
    </>
  );
}
