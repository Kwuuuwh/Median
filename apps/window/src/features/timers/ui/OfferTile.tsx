import { openItem } from "@/features/items/model/open";
import { ItemTile } from "@/features/items/ui/ItemTile";
import { cn } from "@/shared/lib/cn";
import { Money } from "@/shared/ui/money";
import type { ShelfLine } from "../model/shelf";

/** One vendor offer: its price and the copies the account holds; a click opens the item card. */
export function OfferTile({
  line,
  wallet,
}: {
  line: ShelfLine;
  /** The account's amount of the offer's first currency; null when it is not tracked. */
  wallet: number | null;
}) {
  return (
    <ItemTile
      item={line.offer.item}
      image={line.image}
      name={line.name}
      held={line.held}
      mastered={line.mastered}
      known={line.known}
      onOpen={() => openItem(line.offer.item)}
      meta={
        <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-body">
          {line.offer.prices.map((price, index) => (
            <Money
              key={price.currency}
              currency={price.currency}
              amount={price.amount}
              className={cn(
                index === 0 ? "font-medium text-ink" : "text-soft",
                index === 0 && wallet !== null && price.amount > wallet && "opacity-50",
              )}
            />
          ))}
        </div>
      }
    />
  );
}
