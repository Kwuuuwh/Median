import type { Relic, Slot } from "@median/ipc";
import { openItem } from "@/features/items/model/open";
import { RARITY_FILL } from "@/features/items/ui/rarity";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Money } from "@/shared/ui/money";
import { Tooltip } from "@/shared/ui/tooltip";
import { keptSteps } from "../model/rows";

/**
 * One relic reward: picture highlighted with the held count when held, greyed otherwise, and a
 * rarity-coloured bar; a click opens the item card.
 */
function Reward({ slot }: { slot: Slot }) {
  const held = slot.own > 0;
  return (
    <Tooltip hint={slot.name}>
      <button
        type="button"
        onClick={(event) => {
          event.stopPropagation();
          openItem(slot.item);
        }}
        className="grid gap-1"
      >
        <span
          className={cn(
            "relative aspect-square overflow-hidden rounded-control bg-sunken ring-1 ring-hairline ring-inset",
            held && "bg-live/10 ring-live/45",
          )}
        >
          <ItemPicture
            image={slot.image}
            className={cn("size-full p-1", !held && "opacity-35 grayscale")}
          />
          {held && (
            <b className="absolute right-0.5 bottom-0.5 rounded-full bg-live px-1 font-bold text-canvas text-caption tabular-nums">
              {slot.own}
            </b>
          )}
        </span>
        <span className={cn("h-1 rounded-full", RARITY_FILL[slot.rarity] ?? "bg-dim")} />
      </button>
    </Tooltip>
  );
}

/**
 * Tile of one relic: average platinum and ducats per run of each held refinement, next to all its
 * rewards; the header opens the relic's card.
 */
export function RelicTile({ relic }: { relic: Relic }) {
  const t = useT();
  const kept = keptSteps(relic);
  const across = relic.holds.length % 4 === 0 && relic.holds.length % 3 !== 0 ? 4 : 3;
  return (
    <article className="flex flex-col gap-2.5 rounded-panel border border-line bg-surface p-3 transition-colors hover:border-dim">
      <button
        type="button"
        onClick={() => openItem(kept[0]?.item ?? relic.base)}
        className="flex items-center gap-2 text-left"
      >
        <ItemPicture image={relic.image} className="size-9 shrink-0" />
        <span className="min-w-0 flex-1 truncate font-medium text-body text-soft hover:text-accent">
          {relic.name}
        </span>
        {relic.vaulted && (
          <span className="shrink-0 rounded-full bg-duc/12 px-1.5 py-0.5 text-caption text-duc">
            {t("relic-vaulted")}
          </span>
        )}
      </button>
      <div className="flex gap-3">
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          {kept.map((step) => (
            <span key={step.item} className="grid leading-tight">
              <span className="flex items-baseline gap-1 text-caption text-dim">
                <span className="truncate">
                  {t("relic-refinement", { refinement: step.refinement })}
                </span>
                <span className="text-muted tabular-nums">×{step.count}</span>
              </span>
              <span className="flex items-center gap-2.5 tabular-nums">
                <Money
                  currency="platinum"
                  amount={step.platinum}
                  tenths
                  className="font-semibold text-plat text-title"
                />
                <Money
                  currency="ducats"
                  amount={step.ducats}
                  tenths
                  className="text-duc text-small"
                />
              </span>
            </span>
          ))}
        </div>
        <div
          className={cn(
            "grid shrink-0 gap-1.5",
            across === 4 ? "w-56 grid-cols-4" : "w-42 grid-cols-3",
          )}
        >
          {relic.holds.map((slot) => (
            <Reward key={slot.item} slot={slot} />
          ))}
        </div>
      </div>
    </article>
  );
}
