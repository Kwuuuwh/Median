import type { Syndicate } from "@median/ipc";
import { ArrowLeft } from "lucide-react";
import { useState } from "react";
import { ITEM_TILE_WIDTH } from "@/features/items/ui/ItemTile";
import { useLocale } from "@/shared/data/app-state";
import { useOwnOrders } from "@/shared/data/market";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { matcher } from "@/shared/lib/search";
import type { Sorting } from "@/shared/lib/sort";
import { Button } from "@/shared/ui/button";
import { EmptyState } from "@/shared/ui/empty-state";
import { FacetMenu } from "@/shared/ui/facet-menu";
import { NumberField, wholeNumber } from "@/shared/ui/number-field";
import { SearchField } from "@/shared/ui/search-field";
import { SortSelect } from "@/shared/ui/sort-select";
import { TileGrid } from "@/shared/ui/tile-grid";
import {
  type Bounds,
  OFFER_FACETS,
  type OfferFacet,
  type OfferOrder,
  offerWay,
  shownOffers,
} from "../model/offers";
import { StandingOfferTile } from "./StandingOfferTile";
import { SyndicateEmblem } from "./SyndicateEmblem";

const ORDERS: readonly OfferOrder[] = ["platinum", "cost"];

/** Bounds parsed from two fields; a field without a valid number leaves its side open. */
function boundsOf(least: string, most: string): Bounds {
  return { least: wholeNumber(least, 0, 1_000_000), most: wholeNumber(most, 0, 1_000_000) };
}

/** A labelled pair of fields with a lower and an upper bound for one value. */
function Range({
  label,
  least,
  most,
  onLeast,
  onMost,
}: {
  label: string;
  least: string;
  most: string;
  onLeast: (value: string) => void;
  onMost: (value: string) => void;
}) {
  const t = useT();
  return (
    <span className="inline-flex items-center gap-2">
      <span className="text-muted text-small">{label}</span>
      <NumberField
        label={t("syndicate-from")}
        value={least}
        onChange={onLeast}
        invalid={least !== "" && wholeNumber(least, 0, 1_000_000) === null}
        min={0}
        width="w-16"
      />
      <NumberField
        label={t("syndicate-to")}
        value={most}
        onChange={onMost}
        invalid={most !== "" && wholeNumber(most, 0, 1_000_000) === null}
        min={0}
        width="w-16"
      />
    </span>
  );
}

/** Offers one syndicate sells for standing, with the user's search, filters and sort order. */
export function SyndicateShelf({
  syndicate,
  onBack,
}: {
  syndicate: Syndicate;
  onBack: () => void;
}) {
  const t = useT();
  const locale = useLocale();
  const [search, setSearch] = useState("");
  const [order, setOrder] = useState<Sorting<OfferOrder>>({ by: "platinum", descending: true });
  const [facets, setFacets] = useState<ReadonlyMap<OfferFacet, boolean>>(
    new Map([["tradable", true]]),
  );
  const [platinum, setPlatinum] = useState(["", ""]);
  const [volume, setVolume] = useState(["", ""]);
  const selling = (useOwnOrders().data ?? []).filter((order) => order.selling);
  const listingOf = new Map(selling.map((order) => [order.item, order]));
  const shown = shownOffers(
    syndicate,
    {
      facets,
      platinum: boundsOf(platinum[0], platinum[1]),
      volume: boundsOf(volume[0], volume[1]),
    },
    order,
    matcher(search),
    new Set(listingOf.keys()),
  );

  return (
    <div className="grid gap-3">
      <div className="flex flex-wrap items-baseline gap-x-4 gap-y-2">
        <Button tone="ghost" size="sm" onClick={onBack}>
          <ArrowLeft />
          {t("syndicates-all")}
        </Button>
        <span className="flex items-center gap-2 self-center font-medium text-body text-ink">
          <SyndicateEmblem tag={syndicate.tag} className="size-6" />
          {t("syndicate-name", { tag: syndicate.tag })}
        </span>
        <span className={cn("text-small", syndicate.title < 0 ? "text-danger" : "text-muted")}>
          {t("syndicate-rank", { rank: syndicate.title })}
        </span>
        <span className="text-muted text-small tabular-nums">
          {t("syndicate-standing", { tag: syndicate.tag })}{" "}
          <span className="text-ink">{formatCount(locale, syndicate.standing)}</span>
        </span>
        {syndicate.leftToday !== null && (
          <span className="text-muted text-small tabular-nums">
            {t("syndicate-today")} {formatCount(locale, syndicate.leftToday)}
          </span>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <SearchField value={search} onChange={setSearch} placeholder={t("shelf-search")} />
        <SortSelect<OfferOrder>
          label={t("shelf-sort")}
          className="w-48"
          value={order}
          onChange={setOrder}
          wayOf={offerWay}
          choices={ORDERS.map((each) => ({
            value: each,
            label: t("syndicate-order", { order: each }),
          }))}
        />
        <FacetMenu facets={OFFER_FACETS} values={facets} onChange={setFacets} className="w-52" />
        <Range
          label={t("syndicate-platinum-range")}
          least={platinum[0]}
          most={platinum[1]}
          onLeast={(value) => setPlatinum([value, platinum[1]])}
          onMost={(value) => setPlatinum([platinum[0], value])}
        />
        <Range
          label={t("syndicate-volume-range")}
          least={volume[0]}
          most={volume[1]}
          onLeast={(value) => setVolume([value, volume[1]])}
          onMost={(value) => setVolume([volume[0], value])}
        />
      </div>
      {shown.length === 0 ? (
        <EmptyState title={t("shelf-nothing")} />
      ) : (
        <TileGrid least={ITEM_TILE_WIDTH}>
          {shown.map((offer) => (
            <StandingOfferTile
              key={`${offer.vendor}:${offer.item}`}
              offer={offer}
              syndicate={syndicate}
              listing={listingOf.get(offer.item)}
            />
          ))}
        </TileGrid>
      )}
    </div>
  );
}
