import type { OwnOrder } from "@median/ipc";
import { Tag } from "lucide-react";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Tooltip } from "@/shared/ui/tooltip";

/** Badge of the account's sell order on a picture: price and quantity; muted while hidden. */
export function ListedBadge({
  listing,
  counted = false,
  compact = false,
}: {
  listing: OwnOrder;
  /** Whether the quantity is shown even when it is 1. */
  counted?: boolean;
  /** Whether the badge is sized for a small picture. */
  compact?: boolean;
}) {
  const t = useT();
  return (
    <Tooltip hint={t(listing.visible ? "inventory-listed" : "inventory-listed-hidden")}>
      <span
        className={cn(
          "inline-flex items-center gap-1 rounded-sm bg-accent/90 font-medium text-canvas tabular-nums",
          compact ? "px-1 text-[0.7rem] leading-tight" : "px-1.5 py-0.5 text-small",
          !listing.visible && "bg-canvas/80 text-muted",
        )}
      >
        <Tag className={compact ? "size-2.5" : "size-3"} />
        {listing.platinum}
        {(counted || listing.quantity > 1) && (
          <span className="opacity-75">×{listing.quantity}</span>
        )}
      </span>
    </Tooltip>
  );
}
