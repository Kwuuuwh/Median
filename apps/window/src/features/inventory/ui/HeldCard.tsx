import type { Good, OwnOrder } from "@median/ipc";
import { Layers } from "lucide-react";
import { openItem } from "@/features/items/model/open";
import { ItemCard } from "@/features/items/ui/ItemCard";
import { ListedBadge } from "@/features/items/ui/ListedBadge";
import { type Translate, useT } from "@/shared/i18n";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Money } from "@/shared/ui/money";
import { Tooltip } from "@/shared/ui/tooltip";
import { freeCount, type HeldLine } from "../model/lines";

/** Number of items a fit line shows pictures of. */
const FACES = 3;

/** Item names the fit line's tooltip lists; the rest are shown as a count. */
const NAMED = 8;

/**
 * Detail text of a line: rank out of max rank with copies still to fuse, ayatan stars, or the
 * riven's weapon; null when there is none.
 */
function detailOf(t: Translate, line: HeldLine): string | null {
  const detail = line.detail;
  if (detail?.kind === "stars") return t("detail-stars", { stars: detail.stars });
  if (detail?.kind === "riven") return detail.revealed?.weapon ?? t("detail-veiled");
  if (line.maxRank !== null && (line.rank !== null || line.toMax !== null)) {
    const rank = t("inventory-rank", { rank: line.rank ?? 0, max: line.maxRank });
    if (line.toMax !== null) return `${rank} · ${t("inventory-to-max", { count: line.toMax })}`;
    return rank;
  }
  return line.rank !== null ? t("detail-rank", { rank: line.rank }) : null;
}

/** Where the copies are installed: item count and free copies, with the item names on hover. */
function FitLine({ line, good }: { line: HeldLine; good: (item: string) => Good | undefined }) {
  const t = useT();
  const names = (line.fitted ?? []).map((item) => good(item)?.name ?? item);
  const hint =
    names.length === 0
      ? t("inventory-equipped")
      : [
          names.slice(0, NAMED).join(", "),
          names.length > NAMED ? t("inventory-more", { count: names.length - NAMED }) : "",
        ]
          .filter(Boolean)
          .join(" ");
  return (
    <Tooltip hint={hint}>
      <span className="relative z-10 inline-flex items-center gap-1.5 justify-self-start text-small text-soft tabular-nums">
        {names.length > 0 ? (
          <span className="flex -space-x-1">
            {(line.fitted ?? []).slice(0, FACES).map((item) => (
              <ItemPicture
                key={item}
                image={good(item)?.image ?? null}
                className="size-5 rounded-full bg-soft/20 ring-2 ring-surface group-hover:ring-raised"
              />
            ))}
          </span>
        ) : (
          <Layers className="size-3.5 text-muted" />
        )}
        {names.length > 0
          ? t("inventory-fitted", { count: names.length })
          : t("inventory-equipped")}
        {line.count > 1 && (
          <span className="text-muted">· {t("inventory-free", { count: freeCount(line) })}</span>
        )}
      </span>
    </Tooltip>
  );
}

/** Inventory line card: count, copy price, rank, where it is installed; opens the item card. */
export function HeldCard({
  line,
  good,
  listing,
}: {
  line: HeldLine;
  good: (item: string) => Good | undefined;
  /** The account's open sell order for the item, if any. */
  listing?: OwnOrder;
}) {
  const t = useT();
  const detail = detailOf(t, line);
  return (
    <ItemCard
      item={line.item}
      image={line.image}
      name={line.name}
      count={line.count}
      mastered={line.built === "mastered"}
      crafted={line.built === "crafted"}
      known={line.known}
      onOpen={() => openItem(line.item, line.variant)}
      vaulted={line.vaulted}
      badge={listing && <ListedBadge listing={listing} />}
      figures={
        <>
          <div className="flex items-center gap-3 text-body">
            {line.tradable && (
              <Money currency="platinum" amount={line.platinum} className="font-medium text-ink" />
            )}
            {line.ducats !== null && (
              <Money currency="ducats" amount={line.ducats} className="text-soft" />
            )}
            {detail && <span className="ml-auto truncate text-muted tabular-nums">{detail}</span>}
          </div>
          {line.equipped && <FitLine line={line} good={good} />}
        </>
      }
    />
  );
}
