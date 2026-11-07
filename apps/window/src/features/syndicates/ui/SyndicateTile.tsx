import type { Syndicate } from "@median/ipc";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { SyndicateEmblem } from "./SyndicateEmblem";

/** Vendor locations of the syndicate joined by ` · `; empty for vendors on every relay. */
function placesOf(syndicate: Syndicate): string {
  const places = new Set(syndicate.offers.flatMap((offer) => (offer.place ? [offer.place] : [])));
  return [...places].join(" · ");
}

/** Syndicate tile: rank, standing, and a summary of its vendor offers; opens the offer list. */
export function SyndicateTile({ syndicate, onOpen }: { syndicate: Syndicate; onOpen: () => void }) {
  const t = useT();
  const locale = useLocale();
  const tradable = syndicate.offers.filter((offer) => offer.tradable).length;
  const places = placesOf(syndicate);
  return (
    <button
      type="button"
      onClick={onOpen}
      className={cn(
        "grid gap-2 rounded-panel border border-line bg-surface p-4 text-left transition-colors hover:border-dim",
        !syndicate.met && "opacity-60",
      )}
    >
      <span className="flex items-center gap-3">
        <SyndicateEmblem tag={syndicate.tag} className="size-11" />
        <span className="grid min-w-0 flex-1 gap-0.5">
          <span className="truncate font-medium text-body text-ink">
            {t("syndicate-name", { tag: syndicate.tag })}
          </span>
          <span
            className={cn(
              "text-small tabular-nums",
              syndicate.title < 0 ? "text-danger" : "text-muted",
            )}
          >
            {syndicate.met ? t("syndicate-rank", { rank: syndicate.title }) : t("syndicate-unmet")}
          </span>
        </span>
      </span>
      <span className="flex items-baseline gap-2">
        <span className="text-muted text-small">
          {t("syndicate-standing", { tag: syndicate.tag })}
        </span>
        <span className="font-display font-semibold text-heading text-ink tabular-nums">
          {formatCount(locale, syndicate.standing)}
        </span>
      </span>
      <span className="flex flex-wrap gap-x-4 text-muted text-small tabular-nums">
        {syndicate.leftToday !== null && (
          <span>
            {t("syndicate-today")} {formatCount(locale, syndicate.leftToday)}
          </span>
        )}
        <span>{t("syndicate-offers", { count: tradable })}</span>
      </span>
      {places && <span className="truncate text-dim text-small">{places}</span>}
    </button>
  );
}
