import type { ReactNode } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount, formatTenths } from "@/shared/lib/format";
import { TileGrid } from "@/shared/ui/tile-grid";
import { runsFor } from "../../model/sources";
import { HeldMark } from "../HeldMark";

/** Small tile for one item source: title, detail, and the chance and average runs if given. */
export function SourceTile({
  picture,
  title,
  held = 0,
  detail,
  chance,
  aside,
  faded = false,
  onOpen,
}: {
  picture?: ReactNode;
  title: ReactNode;
  /** Copies the account holds. */
  held?: number;
  detail?: ReactNode;
  /** Chance in tenths of a percent. */
  chance?: number;
  aside?: ReactNode;
  faded?: boolean;
  onOpen?: () => void;
}) {
  const t = useT();
  const locale = useLocale();
  const body = (
    <>
      <span className="flex min-w-0 items-center gap-2">
        {picture}
        <span className="min-w-0 flex-1 text-body text-ink leading-tight">
          {title} <HeldMark count={held} className="align-middle" />
        </span>
        {aside}
      </span>
      {detail && <span className="min-w-0 text-muted text-small leading-tight">{detail}</span>}
      {chance !== undefined && (
        <span className="mt-auto flex items-baseline gap-2 tabular-nums">
          <span className="font-semibold text-ink text-title">
            {t("card-chance", { percent: formatTenths(locale, chance) })}
          </span>
          <span className="text-muted text-small">
            {t("item-runs", { runs: formatCount(locale, runsFor(chance)) })}
          </span>
        </span>
      )}
    </>
  );
  const frame = cn(
    "grid min-w-0 grid-cols-[minmax(0,1fr)] content-start gap-1.5 rounded-control border border-line bg-canvas/40 p-2.5 text-left",
    faded && "opacity-55",
  );
  return onOpen ? (
    <button
      type="button"
      onClick={onOpen}
      className={cn(frame, "transition-colors hover:border-dim")}
    >
      {body}
    </button>
  ) : (
    <div className={frame}>{body}</div>
  );
}

/** Grid of source tiles, as many across as fit; with `onMore`, a last tile shows the `hidden`
 * count and expands them. */
export function SourceGrid({
  children,
  hidden = 0,
  onMore,
}: {
  children: ReactNode;
  hidden?: number;
  onMore?: () => void;
}) {
  const t = useT();
  return (
    <TileGrid least={176} className="gap-2">
      {children}
      {hidden > 0 && onMore && (
        <button
          type="button"
          onClick={onMore}
          className="rounded-control border border-line border-dashed p-2.5 text-muted text-small transition-colors hover:border-dim hover:text-ink"
        >
          {t("card-more", { count: hidden })}
        </button>
      )}
    </TileGrid>
  );
}
