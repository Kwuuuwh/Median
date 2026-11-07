import { commands, type Listing, type Piece, unwrap } from "@median/ipc";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, Tag } from "lucide-react";
import { useState } from "react";
import { useAppState } from "@/shared/data/app-state";
import { keys } from "@/shared/data/keys";
import { useListings, useOwnOrders } from "@/shared/data/market";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { MOST_COPIES } from "@/shared/lib/order";
import { Button } from "@/shared/ui/button";
import { Money } from "@/shared/ui/money";
import { NumberField, wholeNumber } from "@/shared/ui/number-field";
import { Tooltip } from "@/shared/ui/tooltip";

/** Most copies one trade can include, the slot count of the in-game trade window. */
const MOST_PER_TRADE = 6;

/** Rank of a held copy; null for an item without ranks. */
function rankOf(piece: Piece): number | null {
  return piece.detail && piece.detail.kind !== "stars" ? piece.detail.rank : null;
}

/** Best competing price among in-game players on the same side: the lowest sell order when
 * selling, the highest buy order when buying; at `rank` for a ranked item; null without one. */
function bestRival(rows: readonly Listing[], rank: number | null, selling: boolean): number | null {
  const prices = rows
    .filter(
      (row) =>
        row.selling === selling && row.status === "ingame" && (rank === null || row.rank === rank),
    )
    .map((row) => row.platinum);
  if (prices.length === 0) return null;
  return selling ? Math.min(...prices) : Math.max(...prices);
}

/** Form that places a sell or buy order for an item: rank, quantity, copies per trade and price.
 * A sell order is not limited by the held count, because the inventory data lags the game. */
export function TradeBar({
  item,
  selling,
  copies,
  held,
  price,
  className,
}: {
  item: string;
  selling: boolean;
  /** Held pieces of the item, one per variant the game distinguishes. */
  copies: readonly Piece[];
  /** Copies the inventory reports; for a complete set, the sets the held parts make. */
  held: number;
  /** Market price of one copy, the starting price until the order book loads. */
  price: number | null;
  className?: string;
}) {
  const t = useT();
  const client = useQueryClient();
  const signedIn = useAppState()?.membership.standing === "in";
  const book = useListings(item, "").data;
  const maxRank = book?.maxRank ?? null;
  const bulk = book?.bulk ?? false;
  const heldRank = copies.map(rankOf).find((rank) => rank !== null) ?? 0;
  const [rank, setRank] = useState(String(heldRank));
  const [quantity, setQuantity] = useState("1");
  const [perTrade, setPerTrade] = useState("1");
  const [typed, setPlatinum] = useState<string | null>(null);
  const ranked = maxRank === null ? null : wholeNumber(rank, 0, maxRank);
  const rival = bestRival(book?.rows ?? [], ranked, selling);
  const suggested = rival ?? price;
  const platinum = typed ?? (suggested === null ? "" : String(suggested));
  const listed = (useOwnOrders().data ?? []).filter(
    (order) => order.selling === selling && order.item === item,
  );
  const place = useMutation({
    mutationFn: (posting: { quantity: number; platinum: number; lot: number | null }) =>
      unwrap(
        commands.placeOrder({
          item,
          selling,
          platinum: posting.platinum,
          quantity: posting.quantity,
          rank: ranked,
          subtype: null,
          perTrade: posting.lot,
        }),
      ),
    onSuccess: () => void client.invalidateQueries({ queryKey: keys.ordersAll }),
  });
  const count = wholeNumber(quantity, 1, MOST_COPIES);
  const lot = bulk ? wholeNumber(perTrade, 1, MOST_PER_TRADE) : 1;
  const each = wholeNumber(platinum, 1, 100_000);
  const ready =
    count !== null && lot !== null && each !== null && (maxRank === null || ranked !== null);
  const edit = (set: (value: string) => void) => (value: string) => {
    set(value);
    place.reset();
  };

  return (
    <section className={cn("grid content-start gap-2 px-4 py-3", className)}>
      <div className="flex min-h-5 flex-wrap items-center gap-x-3 gap-y-1">
        <span className="font-medium text-body text-soft">
          {t(selling ? "own-selling" : "own-buying")}
        </span>
        {listed.length > 0 && (
          <span className="inline-flex items-center gap-1.5 text-muted text-small">
            <Tag className="size-3.5" />
            {listed.map((order) => (
              <span key={order.id} className="inline-flex items-center gap-1 tabular-nums">
                {order.quantity} ×
                <Money currency="platinum" amount={order.platinum} className="text-soft" />
                {order.perTrade > 1 && <span className="text-plat">×{order.perTrade}</span>}
              </span>
            ))}
          </span>
        )}
      </div>
      <div className="flex items-end gap-2">
        {maxRank !== null && (
          <NumberField
            stacked
            label={t("sell-rank")}
            value={rank}
            onChange={edit(setRank)}
            invalid={ranked === null}
            width="w-12"
            min={0}
            max={maxRank}
          >
            <span className="text-dim tabular-nums">/{maxRank}</span>
          </NumberField>
        )}
        <NumberField
          stacked
          label={t("sell-quantity")}
          value={quantity}
          onChange={edit(setQuantity)}
          invalid={count === null}
          width="w-14"
        >
          {selling && (
            <span className="whitespace-nowrap text-dim tabular-nums">
              {t("sell-held", { count: held })}
            </span>
          )}
        </NumberField>
        {bulk && (
          <NumberField
            stacked
            label={t("sell-per-trade")}
            value={perTrade}
            onChange={edit(setPerTrade)}
            invalid={lot === null}
            width="w-12"
            max={MOST_PER_TRADE}
          />
        )}
        <NumberField
          stacked
          label={t("sell-price")}
          value={platinum}
          onChange={edit(setPlatinum)}
          invalid={platinum !== "" && each === null}
          width="w-full"
          className="min-w-16 flex-1"
        />
        {signedIn ? (
          <Button
            className="w-32 shrink-0"
            disabled={!ready || place.isPending}
            onClick={() => {
              if (count !== null && each !== null && lot !== null) {
                place.mutate({ quantity: count, platinum: each, lot: bulk ? lot : null });
              }
            }}
          >
            {place.isSuccess ? <Check /> : <Tag />}
            {t(place.isSuccess ? "sell-placed" : "sell-place")}
          </Button>
        ) : (
          <Tooltip hint={t("sell-signed-out")}>
            <span className="shrink-0">
              <Button disabled className="w-32">
                <Tag />
                {t("sell-place")}
              </Button>
            </span>
          </Tooltip>
        )}
      </div>
    </section>
  );
}
