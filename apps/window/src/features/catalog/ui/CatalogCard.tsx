import type { Collectible } from "@median/ipc";
import { Lock } from "lucide-react";
import { openItem } from "@/features/items/model/open";
import { ItemCard } from "@/features/items/ui/ItemCard";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { formatCompact } from "@/shared/lib/format";
import { Tooltip } from "@/shared/ui/tooltip";

/**
 * Catalog card of one item: the copies the account holds or, while it holds none, whether it can
 * be built now; the rank it needs while the account lacks it. Clicking it opens the item card.
 */
export function CatalogCard({ row }: { row: Collectible }) {
  const t = useT();
  const locale = useLocale();
  const note =
    row.owned > 0 ? (
      <span className="rounded-sm bg-canvas/80 px-1.5 font-medium text-ink text-small tabular-nums backdrop-blur-sm">
        ×{formatCompact(locale, row.owned)}
      </span>
    ) : (
      row.ready && (
        <span className="rounded-sm bg-canvas/80 px-1.5 text-accent text-small backdrop-blur-sm">
          {t("catalogue-ready")}
        </span>
      )
    );
  return (
    <ItemCard
      item={row.item}
      image={row.image}
      name={row.name}
      count={0}
      mastered={row.mastered === true}
      vaulted={row.vaulted}
      onOpen={() => openItem(row.item)}
      note={note}
      badge={
        row.reachable === false && (
          <Tooltip hint={t("rank-gate", { rank: row.required })}>
            <span className="inline-flex items-center gap-1 rounded-sm bg-canvas/80 px-1.5 text-danger text-small tabular-nums">
              <Lock className="size-3" />
              {row.required}
            </span>
          </Tooltip>
        )
      }
    />
  );
}
