import type { Relic } from "@median/ipc";
import type { ReactNode } from "react";
import { openItem } from "@/features/items/model/open";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Money } from "@/shared/ui/money";
import { HeldMark } from "./HeldMark";
import { ItemLine } from "./ItemLine";
import { ItemThumb } from "./ItemThumb";
import { RARITY_FILL } from "./rarity";

/** Relic card: average platinum per run at one refinement, its rewards, and caller `meta`. */
export function RelicCard({
  relic,
  held,
  step = 0,
  meta,
  wanted,
  focused = false,
}: {
  relic: Relic;
  /** Held count shown beside the name. */
  held: number;
  /** Index in `steps` of the refinement whose run value is shown. */
  step?: number;
  /** Content before the run value, such as the price. */
  meta?: ReactNode;
  wanted: ReadonlySet<string>;
  focused?: boolean;
}) {
  const t = useT();
  const priced = relic.steps[step] ?? relic.steps[0];
  return (
    <article
      id={`relic-${relic.base}`}
      className={cn(
        "flex scroll-m-4 flex-col rounded-panel border border-line bg-surface transition-shadow",
        focused && "ring-2 ring-accent/60",
      )}
    >
      <header className="flex gap-3 p-3">
        <ItemThumb image={relic.image} />
        <div className="grid min-w-0 flex-1 content-start gap-1.5 pt-0.5">
          <h3 className="flex flex-wrap items-center gap-x-2 font-medium text-body text-ink">
            <button
              type="button"
              onClick={() => openItem(priced?.item ?? relic.base)}
              className="text-left hover:text-accent"
            >
              {relic.name}
            </button>
            <HeldMark count={held} />
          </h3>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-body">
            {meta}
            <span className="inline-flex items-center gap-1">
              <Money
                currency="platinum"
                amount={priced?.platinum ?? 0}
                tenths
                className="text-soft"
              />
              <span className="text-muted text-small">{t("shelf-per-run")}</span>
            </span>
          </div>
        </div>
      </header>
      <ul className="border-hairline border-t px-3 py-1.5">
        {relic.holds.map((slot) => (
          <ItemLine
            key={slot.item}
            image={slot.image}
            name={slot.name}
            own={slot.own}
            wanted={wanted.has(slot.item)}
            platinum={slot.platinum}
            onOpen={() => openItem(slot.item)}
            lead={
              <span
                className={cn(
                  "size-1.5 shrink-0 rounded-full",
                  RARITY_FILL[slot.rarity] ?? "bg-dim",
                )}
              />
            }
          />
        ))}
      </ul>
    </article>
  );
}
