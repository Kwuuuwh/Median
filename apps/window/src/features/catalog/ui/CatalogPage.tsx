import { useState } from "react";
import { CARD_HEIGHT, CARD_WIDTH } from "@/features/items/ui/ItemCard";
import { useLocale } from "@/shared/data/app-state";
import { useCollectibles, useTaxonomy } from "@/shared/data/catalog";
import { useFavourites } from "@/shared/data/favourites";
import { useItemSearch } from "@/shared/data/search";
import { useT } from "@/shared/i18n";
import { useKept } from "@/shared/lib/use-kept";
import { EmptyState } from "@/shared/ui/empty-state";
import { FacetMenu } from "@/shared/ui/facet-menu";
import { SearchField } from "@/shared/ui/search-field";
import { Select } from "@/shared/ui/select";
import { VirtualGrid } from "@/shared/ui/virtual-grid";
import { CATALOG_FACETS, type CatalogFacets, shelve } from "../model/shelf";
import { CatalogCard } from "./CatalogCard";

/** Class choice for every class. */
const ALL = "?all";

/**
 * Every item the catalog lists, narrowed by class, search and yes/no filters, sorted by name;
 * each card opens the item card.
 */
export function CatalogPage() {
  const t = useT();
  const locale = useLocale();
  const rows = useCollectibles().data;
  const taxonomy = useTaxonomy().data;
  const favourites = useFavourites().data;
  const [picked, setPicked] = useKept("catalog.class", ALL);
  const [facets, setFacets] = useKept<CatalogFacets>("catalog.facets", new Map());
  const search = useItemSearch("catalog.search", rows?.map((row) => row.item) ?? []);
  const [scroller, setScroller] = useState<HTMLDivElement | null>(null);
  if (!rows) return null;

  const classes = (taxonomy?.classes ?? [])
    .filter((each) => rows.some((row) => row.class === each.slug))
    .sort((one, other) => one.ord - other.ord);
  const className = classes.some((each) => each.slug === picked) ? picked : ALL;
  const shown = shelve(rows, {
    className: className === ALL ? null : className,
    matches: search.matches,
    facets,
    favourite: (item) => favourites?.has(item) ?? false,
    locale,
  });

  return (
    <div className="flex min-w-0 flex-1 flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-2 border-line border-b px-6 py-3">
        <Select
          label={t("catalogue-class")}
          className="w-48"
          value={className}
          onChange={setPicked}
          choices={[
            { value: ALL, label: t("filter-class-any") },
            ...classes.map((each) => ({ value: each.slug, label: each.name })),
          ]}
        />
        <SearchField
          {...search.field}
          placeholder={t("catalogue-search")}
          className="w-56 min-w-36 shrink"
        />
        <FacetMenu facets={CATALOG_FACETS} values={facets} onChange={setFacets} className="w-48" />
        <span className="ml-auto text-dim text-small tabular-nums">
          {t("filter-shown", { shown: shown.length, count: rows.length })}
        </span>
      </div>
      <div ref={setScroller} className="relative min-h-0 flex-1 overflow-y-auto px-6 pt-4 pb-16">
        {shown.length === 0 ? (
          <EmptyState title={t("catalogue-none")} />
        ) : (
          <VirtualGrid
            items={shown}
            scroller={scroller}
            least={CARD_WIDTH}
            estimate={CARD_HEIGHT}
            keyOf={(row) => row.item}
          >
            {(row) => <CatalogCard row={row} />}
          </VirtualGrid>
        )}
      </div>
    </div>
  );
}
