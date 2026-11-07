import type { Position } from "@median/ipc";
import { Trash2 } from "lucide-react";
import { useState } from "react";
import { openItem } from "@/features/items/model/open";
import { Action } from "@/features/orders/ui/OrderTile";
import { useReleasePosition } from "@/shared/data/trading";
import { useT } from "@/shared/i18n";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Money } from "@/shared/ui/money";

/** Expected margin of selling all copies at the lowest in-game sell price; null without one. */
function marginOf(position: Position): number | null {
  return position.ingame === null ? null : (position.ingame - position.basis) * position.quantity;
}

/** A position held for resale: cost per copy, cheapest in-game seller, the account's own asking
 * price, expected margin, and a two-click button that takes the position away from the agent. */
export function PositionTile({ position }: { position: Position }) {
  const t = useT();
  const release = useReleasePosition();
  const [isConfirming, setConfirming] = useState(false);
  const margin = marginOf(position);
  return (
    <article
      onMouseLeave={() => setConfirming(false)}
      className="flex flex-col gap-3 rounded-panel border border-line bg-surface p-3 transition-colors hover:border-dim"
    >
      <header className="flex min-w-0 items-start gap-2.5">
        <button
          type="button"
          disabled={!position.item}
          onClick={() => openItem(position.item)}
          className="flex min-w-0 flex-1 items-center gap-2.5 text-left"
        >
          <ItemPicture
            image={position.image}
            className="size-11 shrink-0 rounded-control bg-sunken p-0.5"
          />
          <span className="grid min-w-0 gap-0.5">
            <span className="line-clamp-2 text-body text-ink leading-snug hover:text-accent">
              {position.name}
            </span>
            <span className="flex items-center gap-2 text-muted text-small tabular-nums">
              {position.quantity > 1 && <span>×{position.quantity}</span>}
              <span>{t("span-days", { days: position.days })}</span>
            </span>
            {margin !== null && (
              <span className="whitespace-nowrap text-small tabular-nums">
                <span className={margin >= 0 ? "text-live/80" : "text-danger/80"}>
                  ≈ {margin > 0 ? `+${margin}` : margin}
                </span>
                <span className="ml-1 text-dim">{t("agent-expected")}</span>
              </span>
            )}
          </span>
        </button>
        <Action
          hint={t(isConfirming ? "agent-release-confirm" : "agent-release")}
          onClick={() => {
            if (!isConfirming) {
              setConfirming(true);
              return;
            }
            setConfirming(false);
            release.mutate({ slug: position.slug, variant: position.variant });
          }}
          disabled={release.isPending}
          danger
        >
          <Trash2 className={isConfirming ? "text-danger" : undefined} />
        </Action>
      </header>
      <div className="grid grid-cols-3 gap-2 border-hairline border-t pt-2.5 text-small">
        <span className="grid gap-0.5">
          <span className="text-caption text-dim">{t("trade-basis")}</span>
          <Money currency="platinum" amount={position.basis} className="text-soft" />
        </span>
        <span className="grid gap-0.5">
          <span className="text-caption text-dim">{t("trade-ingame")}</span>
          <Money currency="platinum" amount={position.ingame} className="text-soft" />
        </span>
        <span className="grid gap-0.5">
          <span className="text-caption text-dim">{t("trade-asking")}</span>
          <Money currency="platinum" amount={position.asking} className="text-soft" />
        </span>
      </div>
    </article>
  );
}
