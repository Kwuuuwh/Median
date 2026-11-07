import { Inbox } from "lucide-react";
import { ToggleGroup } from "radix-ui";
import { useState } from "react";
import { useAppState, useLocale } from "@/shared/data/app-state";
import { useCard, useGoods } from "@/shared/data/catalog";
import { useHoldings } from "@/shared/data/holdings";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { CurrencyIcon } from "@/shared/ui/currency-icon";
import { EmptyLine } from "@/shared/ui/empty-state";
import { Money } from "@/shared/ui/money";
import { Segmented } from "@/shared/ui/segmented";
import { Tooltip } from "@/shared/ui/tooltip";
import { relicLabel } from "../../model/relic";
import {
  dropSources,
  isReachable,
  relicSources,
  SOURCE_KINDS,
  sourceCounts,
} from "../../model/sources";
import { ItemThumb } from "../ItemThumb";
import { RARITY_FILL } from "../rarity";
import { VendorCost } from "../VendorCost";
import { SourceGrid, SourceTile } from "./SourceTile";

/** Tiles a source kind shows before it is expanded. */
const SHOWN = 8;

type Refinement = "intact" | "radiant";

/** Sources of an item as tiles, one kind at a time: relics, missions, enemies, vendors, labs. */
export function ItemSources({ item, onOpen }: { item: string; onOpen: (item: string) => void }) {
  const t = useT();
  const locale = useLocale();
  const [picked, setPicked] = useState<string | null>(null);
  const [refinement, setRefinement] = useState<Refinement>("intact");
  const [all, setAll] = useState(false);
  const own = useCard(item).data;
  const builder =
    own && sourceCounts(own).size === 0
      ? (own.recipes.find((recipe) => recipe.blueprint !== item)?.blueprint ?? null)
      : null;
  const built = useCard(builder).data;
  const card = builder ? built : own;
  const found = card?.found ?? [];
  const relicGoods = useGoods([...new Set(found.map((reward) => reward.item))].sort()).data ?? [];
  const pieces = useHoldings().data?.pieces ?? [];
  const shelved = new Set(
    (useAppState()?.shelves ?? []).flatMap((shelf) => shelf.offers.map((offer) => offer.item)),
  );
  if (!card) return null;

  const counts = sourceCounts(card);
  const kinds = SOURCE_KINDS.filter((kind) => counts.has(kind));
  if (kinds.length === 0) return <EmptyLine icon={Inbox}>{t("card-empty")}</EmptyLine>;
  const kind: string = picked && counts.has(picked) ? picked : kinds[0];

  const held = new Map<string, number>();
  for (const piece of pieces) held.set(piece.item, (held.get(piece.item) ?? 0) + piece.count);
  const vaulted = new Set(relicGoods.filter((good) => good.vaulted).map((good) => good.item));

  let tiles: React.ReactNode[] = [];
  if (kind === "relic") {
    tiles = relicSources(
      found,
      (key) => held.get(key) ?? 0,
      (key) => vaulted.has(key),
      refinement,
      (key) => shelved.has(key),
    ).map((source) => (
      <SourceTile
        key={source.name}
        picture={<ItemThumb image={source.image} size="sm" />}
        title={relicLabel(source.name)}
        held={source.held}
        aside={
          <span className="inline-flex items-center gap-1.5">
            {source.offered && (
              <Tooltip hint={t("trader-name", { vendor: "varzia" })}>
                <span className="inline-flex">
                  <CurrencyIcon currency="aya" />
                </span>
              </Tooltip>
            )}
            <span className={cn("size-1.5 rounded-full", RARITY_FILL[source.rarity] ?? "bg-dim")} />
          </span>
        }
        detail={source.vaulted && !source.offered ? t("relic-vaulted") : undefined}
        chance={source.chance[refinement] ?? source.chance.intact ?? 0}
        faded={!isReachable(source)}
        onOpen={() => onOpen(source.opens)}
      />
    ));
  } else if (kind === "vendor") {
    tiles = card.sold.map((sale) => (
      <SourceTile
        key={`${sale.vendor}:${sale.currency}:${sale.pack}`}
        title={sale.vendor}
        detail={
          <span className="flex flex-col gap-0.5">
            {(sale.pack || sale.place) && (
              <span>{[sale.pack, sale.place].filter(Boolean).join(" · ")}</span>
            )}
            <span className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-soft">
              {sale.cost !== null && (
                <VendorCost cost={sale.cost} currency={sale.currency} coin={sale.coin} />
              )}
              {sale.credits !== null && <Money currency="credits" amount={sale.credits} />}
              {sale.rank !== null && (
                <span className="text-muted">{t("card-rank", { rank: sale.rank })}</span>
              )}
            </span>
          </span>
        }
      />
    ));
  } else if (kind === "lab") {
    tiles = card.research.map((research) => (
      <SourceTile
        key={research.lab}
        title={research.lab}
        detail={`${formatCount(locale, research.credits)} ${t("currency-short", { currency: "credits" })}`}
      />
    ));
  } else {
    tiles = dropSources(card.drops.filter((drop) => drop.kind === kind)).map((drop) => (
      <SourceTile
        key={`${drop.place}:${drop.rotation}:${drop.stage}:${drop.refinement}`}
        title={drop.place}
        detail={[
          drop.mission,
          drop.planet,
          drop.rotation && t("card-rotation", { rotation: drop.rotation }),
          drop.refinement &&
            drop.refinement !== "intact" &&
            t("relic-refinement", { refinement: drop.refinement }),
        ]
          .filter(Boolean)
          .join(" · ")}
        chance={drop.chance}
      />
    ));
  }
  const shown = all ? tiles.length : SHOWN;

  return (
    <div className="grid gap-3">
      <div className="flex flex-wrap items-center gap-2">
        {
          <ToggleGroup.Root
            type="single"
            value={kind}
            onValueChange={(value) => {
              if (!value) return;
              setPicked(value);
              setAll(false);
            }}
            className="flex flex-wrap gap-1.5"
          >
            {kinds.map((each) => (
              <ToggleGroup.Item
                key={each}
                value={each}
                className="inline-flex h-7 items-center gap-1.5 rounded-full border border-line px-3 text-muted text-small transition-colors hover:text-ink data-[state=on]:border-accent/60 data-[state=on]:bg-accent/10 data-[state=on]:text-ink"
              >
                {t("card-source-kind", { kind: each })}
                <span className="text-dim tabular-nums">{counts.get(each)}</span>
              </ToggleGroup.Item>
            ))}
          </ToggleGroup.Root>
        }
        {kind === "relic" && (
          <Segmented<Refinement>
            label={t("item-refinement")}
            className="ml-auto h-7"
            value={refinement}
            onChange={setRefinement}
            segments={(["intact", "radiant"] as const).map((each) => ({
              value: each,
              label: t("relic-refinement", { refinement: each }),
            }))}
          />
        )}
      </div>
      <SourceGrid hidden={tiles.length - shown} onMore={() => setAll(true)}>
        {tiles.slice(0, shown)}
      </SourceGrid>
    </div>
  );
}
