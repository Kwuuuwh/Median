import { openItem } from "@/features/items/model/open";
import { isSettled } from "@/features/items/model/standing";
import { FavouriteStar } from "@/features/items/ui/FavouriteStar";
import { HeldMark } from "@/features/items/ui/HeldMark";
import { ItemLine } from "@/features/items/ui/ItemLine";
import { ItemThumb } from "@/features/items/ui/ItemThumb";
import { PartsMeter } from "@/features/items/ui/PartsMeter";
import { RelicTag } from "@/features/items/ui/RelicTag";
import { useT } from "@/shared/i18n";
import { Money } from "@/shared/ui/money";
import type { PrimeSet } from "../model/rotation";

/**
 * Card of a prime set in rotation: parts held, each part's price and the relics that drop it;
 * items open their card, relics call `onRelic`.
 */
export function PrimeSetCard({ set, onRelic }: { set: PrimeSet; onRelic: (base: string) => void }) {
  const t = useT();
  const settled = isSettled(set.standing);
  return (
    <article className="mb-3 break-inside-avoid rounded-panel border border-line bg-surface">
      <header className="flex gap-3 p-3">
        <ItemThumb image={set.image} mastered={set.standing.mastered} />
        <div className="grid min-w-0 flex-1 content-start gap-1.5 pt-0.5">
          <h3 className="flex flex-wrap items-center gap-x-2 font-medium text-body text-ink">
            <button
              type="button"
              onClick={() => openItem(set.built)}
              className="text-left hover:text-accent"
            >
              {set.name}
            </button>
            <HeldMark count={set.standing.held} />
          </h3>
          <div className="flex items-center gap-3">
            {set.standing.parts && <PartsMeter {...set.standing.parts} />}
            <Money currency="platinum" amount={set.platinum} className="text-body text-ink" />
          </div>
        </div>
        <FavouriteStar item={set.built} />
      </header>
      <ul className="border-hairline border-t px-3 py-1.5">
        {set.parts.map((part) => (
          <ItemLine
            key={part.item}
            image={part.image}
            name={part.name}
            own={part.own}
            wanted={!settled && part.own < part.need}
            platinum={part.platinum}
            onOpen={() => openItem(part.item)}
          >
            <span className="flex w-36 shrink-0 flex-wrap justify-end gap-1">
              {part.drops.length > 0 ? (
                part.drops.map((drop) => (
                  <button key={drop.base} type="button" onClick={() => onRelic(drop.base)}>
                    <RelicTag name={drop.name} rarity={drop.rarity} />
                  </button>
                ))
              ) : (
                <span className="text-caption text-dim">{t("shelf-no-drops")}</span>
              )}
            </span>
          </ItemLine>
        ))}
      </ul>
    </article>
  );
}
