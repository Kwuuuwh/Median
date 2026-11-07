import { Hammer } from "lucide-react";
import { openItem } from "@/features/items/model/open";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCompact } from "@/shared/lib/format";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Tooltip } from "@/shared/ui/tooltip";

/** Minimum tile width in pixels. */
export const PLAN_TILE_WIDTH = 200;

/** Estimated tile row height in pixels, used until a row is measured. */
export const PLAN_TILE_HEIGHT = 62;

/** One plan requirement: picture, name and `have/need` count; a click opens the item card. */
export function PlanTile({
  item,
  name,
  image,
  have,
  need,
  startable = false,
}: {
  item: string;
  name: string;
  image: string | null;
  have: number;
  need: number;
  /** Whether the foundry can start building it now. */
  startable?: boolean;
}) {
  const t = useT();
  const locale = useLocale();
  return (
    <button
      type="button"
      onClick={() => openItem(item)}
      className="flex h-full w-full min-w-0 items-center gap-2.5 rounded-panel border border-line bg-surface p-2 text-left transition-colors hover:border-dim hover:bg-raised"
    >
      <span className="size-11 shrink-0 rounded-control bg-sunken">
        <ItemPicture image={image} className="size-full p-0.5" />
      </span>
      <span className="grid min-w-0 flex-1 gap-0.5">
        <span className="line-clamp-2 text-body text-ink leading-snug">{name}</span>
        <span className="flex items-center gap-1.5 text-small tabular-nums">
          <span className={cn(have > 0 ? "text-soft" : "text-muted")}>
            {formatCompact(locale, have)}/{formatCompact(locale, need)}
          </span>
          {startable && (
            <Tooltip hint={t("chain-ready")}>
              <Hammer className="size-3.5 text-live" />
            </Tooltip>
          )}
        </span>
      </span>
    </button>
  );
}
