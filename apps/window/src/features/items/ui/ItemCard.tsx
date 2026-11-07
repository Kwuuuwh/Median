import { Check, Hammer, Lock, PackageOpen } from "lucide-react";
import type { ReactNode } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCompact } from "@/shared/lib/format";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Tooltip } from "@/shared/ui/tooltip";
import { FavouriteStar } from "./FavouriteStar";

/** Minimum card width in pixels. */
export const CARD_WIDTH = 152;

/** Estimated card height in pixels, used by virtual grids until a row is measured. */
export const CARD_HEIGHT = 250;

/**
 * Square item card: the picture with status marks over it, then the name and a line of values;
 * clicking the card opens the item.
 */
export function ItemCard({
  item,
  image,
  name,
  count,
  mastered = false,
  crafted = false,
  known = true,
  vaulted = false,
  figures,
  badge,
  note,
  onOpen,
}: {
  item: string;
  image: string | null;
  name: string;
  /** Copies held; shown over the picture when more than one. */
  count: number;
  mastered?: boolean;
  /** Whether the item this one builds is held. */
  crafted?: boolean;
  /** Whether the item is in the catalog; an unknown one gets a placeholder, no star, no card. */
  known?: boolean;
  /** Whether the item is vaulted; marked only while it is neither crafted nor mastered. */
  vaulted?: boolean;
  /** Content under the name, such as the price. */
  figures?: ReactNode;
  /** Badge in the picture's lower right corner, such as an open order for the item. */
  badge?: ReactNode;
  /** Mark in the picture's upper left corner, shown while the count is not. */
  note?: ReactNode;
  onOpen?: () => void;
}) {
  const t = useT();
  const locale = useLocale();
  const state = mastered
    ? { icon: Check, hint: t("item-mastered"), tone: "text-live" }
    : crafted
      ? { icon: Hammer, hint: t("item-crafted"), tone: "text-soft" }
      : null;
  return (
    <article className="group relative flex flex-col overflow-hidden rounded-panel border border-line bg-surface transition-colors hover:border-dim hover:bg-raised">
      <div className="relative aspect-square overflow-hidden bg-sunken/60">
        {known ? (
          <ItemPicture image={image} className="absolute inset-0 size-full p-4" />
        ) : (
          <span className="flex size-full items-center justify-center text-dim">
            <PackageOpen className="size-8" />
          </span>
        )}
        {count > 1 ? (
          <span className="absolute top-2 left-2 rounded-sm bg-canvas/80 px-1.5 font-medium text-ink text-small tabular-nums backdrop-blur-sm">
            ×{formatCompact(locale, count)}
          </span>
        ) : (
          note && <span className="absolute top-2 left-2 z-10">{note}</span>
        )}
        {known && (
          <span className="absolute top-1 right-1 z-10 flex items-center">
            <FavouriteStar item={item} quiet />
            {state && (
              <Tooltip hint={state.hint}>
                <span className={cn("inline-flex size-7 items-center justify-center", state.tone)}>
                  <state.icon className="size-4" />
                </span>
              </Tooltip>
            )}
          </span>
        )}
        {badge && <span className="absolute right-2 bottom-2 z-10">{badge}</span>}
        {!state && vaulted && (
          <Tooltip hint={t("relic-vaulted")}>
            <span className="absolute bottom-2 left-2 z-10 text-dim">
              <Lock className="size-3.5" />
            </span>
          </Tooltip>
        )}
      </div>
      <div className="grid gap-1 px-2.5 pt-2 pb-2.5">
        <h3 className="h-10 overflow-hidden text-body text-ink leading-tight">
          {onOpen && known ? (
            <button
              type="button"
              onClick={onOpen}
              className="text-left after:absolute after:inset-0 after:rounded-panel"
            >
              <span className="line-clamp-2">{name}</span>
            </button>
          ) : (
            <span className="line-clamp-2">{name}</span>
          )}
        </h3>
        {figures}
      </div>
    </article>
  );
}
