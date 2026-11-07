import { useCard } from "@/shared/data/catalog";
import { useHoldings } from "@/shared/data/holdings";
import { useQuotes } from "@/shared/data/market";
import { cn } from "@/shared/lib/cn";
import { Money } from "@/shared/ui/money";
import { ItemThumb } from "../ItemThumb";
import { RARITY_FILL } from "../rarity";
import { SourceGrid, SourceTile } from "./SourceTile";

/** Rewards a relic can drop, as tiles; each tile opens the reward's card. */
export function ItemContents({ item, onOpen }: { item: string; onOpen: (item: string) => void }) {
  const inside = useCard(item).data?.inside ?? [];
  const pieces = useHoldings().data?.pieces ?? [];
  const quotes = useQuotes(inside.map((reward) => reward.item)).data;
  const held = (key: string) =>
    pieces.filter((piece) => piece.item === key).reduce((sum, piece) => sum + piece.count, 0);
  return (
    <SourceGrid>
      {inside.map((reward) => (
        <SourceTile
          key={reward.item}
          picture={<ItemThumb image={reward.image} size="sm" />}
          title={reward.name}
          held={held(reward.item)}
          aside={
            <span className={cn("size-1.5 rounded-full", RARITY_FILL[reward.rarity] ?? "bg-dim")} />
          }
          detail={
            <Money
              currency="platinum"
              amount={quotes?.get(reward.item) ?? null}
              className="text-soft"
            />
          }
          chance={reward.chance}
          onOpen={() => onOpen(reward.item)}
        />
      ))}
    </SourceGrid>
  );
}
