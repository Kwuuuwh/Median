import type { OwnOrder, StandingOffer, Syndicate } from "@median/ipc";
import { openItem } from "@/features/items/model/open";
import { ItemTile } from "@/features/items/ui/ItemTile";
import { ListedBadge } from "@/features/items/ui/ListedBadge";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount, formatTenths } from "@/shared/lib/format";
import { Money } from "@/shared/ui/money";
import { isAffordable, isOpened } from "../model/offers";

/** A number with one decimal below 10, rounded to a whole number from 10 up. */
function spell(locale: string, value: number): string {
  return value < 10
    ? formatTenths(locale, Math.round(value * 10))
    : formatCount(locale, Math.round(value));
}

/** One syndicate offer: price, trades per day, standing cost, and the account's sell order. */
export function StandingOfferTile({
  offer,
  syndicate,
  listing,
}: {
  offer: StandingOffer;
  syndicate: Syndicate;
  listing?: OwnOrder;
}) {
  const t = useT();
  const locale = useLocale();
  const opened = isOpened(offer, syndicate);
  const affordable = isAffordable(offer, syndicate);
  return (
    <ItemTile
      item={offer.item}
      image={offer.image}
      name={offer.name}
      held={offer.owned}
      mastered={false}
      onOpen={() => openItem(offer.item)}
      meta={
        <div className="grid gap-1">
          {offer.tradable ? (
            <span className="flex flex-wrap items-center gap-x-3 text-muted text-small tabular-nums">
              <Money
                currency="platinum"
                amount={offer.platinum}
                className="font-medium text-body text-ink"
              />
              {listing && <ListedBadge listing={listing} />}
              {offer.perDay !== null && (
                <span>{t("syndicate-per-day", { count: spell(locale, offer.perDay) })}</span>
              )}
              {offer.days !== null && (
                <span>{t("syndicate-days", { days: spell(locale, offer.days) })}</span>
              )}
            </span>
          ) : (
            <span className="text-dim text-small">{t("syndicate-untradable")}</span>
          )}
          <span className="flex flex-wrap items-center gap-x-3 text-small tabular-nums">
            <span className={cn(affordable ? "text-soft" : "text-dim")}>
              {formatCount(locale, offer.cost)}{" "}
              <span className="text-dim">{t("syndicate-standing", { tag: syndicate.tag })}</span>
            </span>
            {!opened && offer.rank !== null && (
              <span className="text-warn">{t("syndicate-needs-rank", { rank: offer.rank })}</span>
            )}
          </span>
        </div>
      }
    />
  );
}
