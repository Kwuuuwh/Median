import type { Masterable } from "@median/ipc";
import { Bookmark, BookmarkCheck, Check, Lock } from "lucide-react";
import { openItem } from "@/features/items/model/open";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCompact } from "@/shared/lib/format";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Tooltip } from "@/shared/ui/tooltip";

/** Minimum tile width in pixels. */
export const TILE_WIDTH = 150;

/** Estimated tile row height in pixels, used until a row is measured. */
export const TILE_HEIGHT = 158;

/**
 * Square tile of an item that gives mastery: the picture with the mastery it still gives, its
 * name, and a plan toggle.
 */
export function MasteryTile({
  item,
  rank,
  planned,
  onPlan,
  onPreview,
}: {
  item: Masterable;
  /** The account's mastery rank; an item that requires a higher rank is shown locked. */
  rank: number;
  planned: boolean;
  onPlan: (on: boolean) => void;
  /** Receives the mastery the item would add while hovered, and 0 when the pointer leaves. */
  onPreview: (points: number) => void;
}) {
  const t = useT();
  const locale = useLocale();
  const mastered = item.stage === "mastered";
  const locked = item.required > rank && !mastered;
  return (
    <article
      onPointerEnter={() => onPreview(item.remaining)}
      onPointerLeave={() => onPreview(0)}
      className="group relative flex flex-col overflow-hidden rounded-panel border border-line bg-surface transition-colors hover:border-dim hover:bg-raised"
    >
      <div className="relative aspect-[5/4] bg-sunken/60">
        <ItemPicture
          image={item.image}
          className={cn("absolute inset-0 size-full p-3", mastered && "opacity-50")}
        />
        {mastered ? (
          <Check className="absolute top-2 left-2 size-4 text-live" />
        ) : (
          <span className="absolute top-2 left-2 rounded-sm bg-canvas/80 px-1.5 font-medium text-live text-small tabular-nums backdrop-blur-sm">
            +{formatCompact(locale, item.remaining)}
          </span>
        )}
        {locked && (
          <Tooltip hint={t("rank-gate", { rank: item.required })}>
            <span className="absolute bottom-2 left-2 z-10 inline-flex items-center gap-1 rounded-sm bg-canvas/80 px-1.5 text-danger text-small tabular-nums">
              <Lock className="size-3" />
              {item.required}
            </span>
          </Tooltip>
        )}
        {!mastered && (
          <Tooltip hint={t(planned ? "mastery-plan-remove" : "mastery-plan-add")}>
            <button
              type="button"
              aria-pressed={planned}
              aria-label={t(planned ? "mastery-plan-remove" : "mastery-plan-add")}
              onClick={() => onPlan(!planned)}
              className={cn(
                "absolute top-1.5 right-1.5 z-10 rounded-control p-1 transition-colors hover:text-ink",
                planned ? "text-accent" : "text-dim opacity-0 group-hover:opacity-100",
              )}
            >
              {planned ? <BookmarkCheck className="size-4" /> : <Bookmark className="size-4" />}
            </button>
          </Tooltip>
        )}
      </div>
      <div className="grid gap-1 px-2.5 pt-2 pb-2.5">
        <button
          type="button"
          onClick={() => openItem(item.item)}
          className="truncate text-left font-medium text-body text-ink after:absolute after:inset-0"
        >
          {item.name}
        </button>
      </div>
    </article>
  );
}
