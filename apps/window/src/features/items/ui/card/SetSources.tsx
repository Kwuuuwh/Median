import { useAppState } from "@/shared/data/app-state";
import { useCard, useGoods } from "@/shared/data/catalog";
import { useHoldings } from "@/shared/data/holdings";
import { useT } from "@/shared/i18n";
import { relicLabel } from "../../model/relic";
import { isReachable, relicSources } from "../../model/sources";
import { ItemThumb } from "../ItemThumb";
import { RARITY_TEXT } from "../rarity";
import { WayLine } from "../WayLine";
import { SourceGrid, SourceTile } from "./SourceTile";

function PartTile({
  item,
  name,
  image,
  held,
  onOpen,
}: {
  item: string;
  name: string;
  image: string | null;
  held: number;
  onOpen: (item: string) => void;
}) {
  const t = useT();
  const card = useCard(item).data;
  const found = card?.found ?? [];
  const builder = card?.way
    ? null
    : (card?.recipes.find((recipe) => recipe.blueprint !== item)?.blueprint ?? null);
  const built = useCard(builder).data;
  const way = card?.way ?? built?.way ?? null;
  const relicGoods = useGoods([...new Set(found.map((reward) => reward.item))].sort()).data ?? [];
  const vaulted = new Set(relicGoods.filter((good) => good.vaulted).map((good) => good.item));
  const shelved = new Set(
    (useAppState()?.shelves ?? []).flatMap((shelf) => shelf.offers.map((offer) => offer.item)),
  );
  const best = relicSources(
    found,
    () => 0,
    (key) => vaulted.has(key),
    "intact",
    (key) => shelved.has(key),
  ).find(isReachable);
  return (
    <SourceTile
      picture={<ItemThumb image={image} size="sm" />}
      title={name}
      held={held}
      detail={
        best ? (
          <span className={RARITY_TEXT[best.rarity] ?? "text-muted"}>{relicLabel(best.name)}</span>
        ) : found.length > 0 ? (
          t("relic-vaulted")
        ) : (
          <WayLine way={way} className="flex-wrap" />
        )
      }
      chance={best?.chance.intact}
      onOpen={() => onOpen(item)}
    />
  );
}

/** Each set part as a tile with its best obtainable relic at the intact refinement. */
export function SetSources({
  parts,
  onOpen,
}: {
  parts: readonly { item: string; name: string; image: string | null }[];
  onOpen: (item: string) => void;
}) {
  const pieces = useHoldings().data?.pieces ?? [];
  const held = (item: string) =>
    pieces.filter((piece) => piece.item === item).reduce((sum, piece) => sum + piece.count, 0);
  return (
    <SourceGrid>
      {parts.map((part) => (
        <PartTile key={part.item} {...part} held={held(part.item)} onOpen={onOpen} />
      ))}
    </SourceGrid>
  );
}
