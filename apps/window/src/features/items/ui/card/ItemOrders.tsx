import type { MessageId } from "@median/i18n";
import {
  ArrowDown,
  ArrowUp,
  Check,
  CircleAlert,
  Copy,
  Inbox,
  LoaderCircle,
  RefreshCw,
  RotateCcw,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useListings } from "@/shared/data/market";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { useNow } from "@/shared/lib/use-now";
import { Button } from "@/shared/ui/button";
import { EmptyLine } from "@/shared/ui/empty-state";
import { Money } from "@/shared/ui/money";
import {
  atRank,
  firstSort,
  type Order,
  type OrderKey,
  type OrderSort,
  pickKey,
  type RankPick,
  sortOrders,
} from "../../model/orders";

/**
 * Grid columns shared by a side's header and rows: status, player, reputation, price (with copies
 * per trade for a bulk item), quantity, copy icon.
 */
function columns(bulk: boolean): string {
  return cn(
    "grid items-center",
    bulk
      ? "grid-cols-[3.75rem_minmax(0,1fr)_3.5rem_5rem_3.5rem_1rem] gap-x-2"
      : "grid-cols-[3.75rem_minmax(0,1fr)_4.5rem_4rem_3.5rem_1rem] gap-x-3",
  );
}

/** Orders a side shows before it is expanded. */
const SHOWN = 12;

/** Refetch interval of the order book while the card is open, in milliseconds. */
const LIVE_MS = 15_000;

/** How long a copied row stays highlighted, in milliseconds. */
const COPIED_MS = 1400;

/** Label and colour of each player status; any other status counts as offline. */
const STATUS: Record<string, { id: MessageId; tone: string }> = {
  ingame: { id: "deal-status-ingame", tone: "text-live" },
  online: { id: "deal-status-online", tone: "text-soft" },
};
const AWAY: { id: MessageId; tone: string } = { id: "deal-status-offline", tone: "text-dim" };

/** Column label of each sort key. */
const KEY_NAME: Record<OrderKey, MessageId> = {
  status: "order-status",
  reputation: "order-reputation",
  quantity: "order-quantity",
  price: "deal-price",
};

/** Sortable column heading; an arrow shows the direction while its key is active. */
function Heading({
  column,
  sort,
  onPick,
  end = false,
}: {
  column: OrderKey;
  sort: OrderSort;
  onPick: (key: OrderKey) => void;
  end?: boolean;
}) {
  const t = useT();
  const descending = sort[column];
  const sorting = descending !== undefined;
  const Way = descending ? ArrowDown : ArrowUp;
  return (
    <button
      type="button"
      onClick={() => onPick(column)}
      className={cn(
        "flex items-center gap-0.5 whitespace-nowrap text-caption transition-colors hover:text-ink",
        end && "justify-end",
        sorting ? "text-ink" : "text-dim",
      )}
    >
      {t(KEY_NAME[column])}
      {sorting && <Way className="size-3 shrink-0 text-accent" />}
    </button>
  );
}

/**
 * One side of the order book as a table, sortable by several keys at once; clicking a row copies
 * its whisper message.
 */
function Side({
  title,
  rows,
  selling,
  bulk,
  onCopied,
}: {
  title: string;
  rows: readonly Order[];
  selling: boolean;
  bulk: boolean;
  /** Called after a message is copied, so the order book can be refetched. */
  onCopied: () => void;
}) {
  const t = useT();
  const locale = useLocale();
  const [sort, setSort] = useState<OrderSort>(() => firstSort(selling));
  const [copied, setCopied] = useState<string | null>(null);
  const [all, setAll] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(null), COPIED_MS);
    return () => clearTimeout(timer);
  }, [copied]);

  const pick = (key: OrderKey) => setSort(pickKey(sort, key));
  const sorted = sortOrders(rows, sort);
  const shown = all ? sorted : sorted.slice(0, SHOWN);
  const grid = columns(bulk);
  return (
    <section className="grid min-w-0 content-start">
      <div className="mb-2 flex flex-wrap items-center gap-x-4 gap-y-1">
        <h5 className="flex items-baseline gap-1.5 font-medium text-body text-ink">
          {title}
          <span className="text-dim text-small tabular-nums">{rows.length}</span>
        </h5>
        {copied && (
          <span className="flex items-center gap-1 text-live text-small" role="status">
            <Check className="size-3.5" />
            {t("order-copied")}
          </span>
        )}
      </div>
      <div className={cn(grid, "border-line border-b px-2 pb-1.5")}>
        <Heading column="status" sort={sort} onPick={pick} />
        <span className="text-caption text-dim">{t("order-player")}</span>
        <Heading column="reputation" sort={sort} onPick={pick} end />
        <Heading column="price" sort={sort} onPick={pick} end />
        <Heading column="quantity" sort={sort} onPick={pick} end />
        <span />
      </div>
      {rows.length === 0 && (
        <EmptyLine icon={Inbox} className="p-2">
          {t("orders-none")}
        </EmptyLine>
      )}
      <ul>
        {shown.map((row) => {
          const key = `${row.name}:${row.platinum}:${row.quantity}`;
          const done = copied === key;
          const status = STATUS[row.status] ?? AWAY;
          return (
            <li key={key}>
              <button
                type="button"
                title={t("item-copy")}
                onClick={() => {
                  void navigator.clipboard.writeText(row.whisper);
                  setCopied(key);
                  onCopied();
                }}
                className={cn(
                  grid,
                  "group h-8 w-full rounded-sm px-2 text-left transition-colors odd:bg-canvas/40 hover:bg-raised",
                  done && "bg-live/10 odd:bg-live/10 hover:bg-live/10",
                )}
              >
                <span className={cn("text-small", status.tone)}>{t(status.id)}</span>
                <span
                  title={row.name}
                  className={cn("truncate text-body", row.reachable ? "text-ink" : "text-muted")}
                >
                  {row.name}
                </span>
                <span className="text-right text-small text-soft tabular-nums">
                  {formatCount(locale, row.reputation)}
                </span>
                <span className="flex items-center justify-end gap-1.5">
                  <Money
                    currency="platinum"
                    amount={row.platinum}
                    className="font-medium text-body text-ink"
                  />
                  {row.perTrade > 1 && (
                    <span
                      title={t("order-lot", {
                        count: row.perTrade,
                        sum: formatCount(locale, row.platinum * row.perTrade),
                      })}
                      className="rounded-sm bg-plat/15 px-1 font-medium text-caption text-plat tabular-nums"
                    >
                      ×{row.perTrade}
                    </span>
                  )}
                </span>
                <span className="text-right text-small text-soft tabular-nums">{row.quantity}</span>
                <span className="flex justify-end">
                  {done ? (
                    <Check aria-label={t("order-copied")} className="size-3.5 text-live" />
                  ) : (
                    <Copy className="size-3.5 text-dim opacity-0 transition-opacity group-hover:opacity-100" />
                  )}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
      {sorted.length > SHOWN && (
        <button
          type="button"
          onClick={() => setAll(!all)}
          className="mt-1 justify-self-start px-2 text-muted text-small hover:text-ink"
        >
          {all ? t("card-less") : t("card-more", { count: sorted.length - SHOWN })}
        </button>
      )}
    </section>
  );
}

/** Age of the order book in seconds, as a button that refetches it. */
function Freshness({
  at,
  fetching,
  onRefresh,
}: {
  at: number;
  fetching: boolean;
  onRefresh: () => void;
}) {
  const t = useT();
  const now = useNow();
  const secs = Math.max(0, now - Math.floor(at / 1000));
  return (
    <button
      type="button"
      onClick={onRefresh}
      className="ml-auto inline-flex items-center gap-1.5 text-dim text-small tabular-nums hover:text-ink"
    >
      <RefreshCw className={cn("size-3.5", fetching && "animate-spin")} />
      {t("orders-fresh", { secs })}
    </button>
  );
}

/**
 * Sell and buy orders of an item at the chart's rank, side by side, each with its own sort; the
 * order book is fetched on mount and every 15 s, with loading, error and empty states.
 */
export function ItemOrders({ item, rank }: { item: string; rank: RankPick }) {
  const t = useT();
  const book = useListings(item, "", LIVE_MS);
  if (book.isPending || (book.isFetching && !book.data)) {
    return (
      <EmptyLine icon={LoaderCircle} className="[&_svg]:animate-spin">
        {t("orders-loading")}
      </EmptyLine>
    );
  }
  if (book.isError) {
    return (
      <div className="flex items-center gap-3">
        <EmptyLine icon={CircleAlert}>{t("orders-failed")}</EmptyLine>
        <Button size="sm" tone="ghost" onClick={() => void book.refetch()}>
          <RotateCcw />
          {t("orders-retry")}
        </Button>
      </div>
    );
  }
  const maxRank = book.data?.maxRank ?? null;
  const rows: Order[] = (book.data?.rows ?? []).filter((row) => atRank(row, rank, maxRank));
  if ((book.data?.rows ?? []).length === 0)
    return <EmptyLine icon={Inbox}>{t("orders-none")}</EmptyLine>;
  const selling = rows.filter((row) => row.selling);
  const buying = rows.filter((row) => !row.selling);
  const bulk = book.data?.bulk ?? false;
  return (
    <div className="grid gap-2">
      <div className="flex">
        <Freshness
          at={book.dataUpdatedAt}
          fetching={book.isFetching}
          onRefresh={() => void book.refetch()}
        />
      </div>
      <div className="grid grid-cols-2 gap-6">
        <Side
          title={t("orders-selling")}
          rows={selling}
          selling
          bulk={bulk}
          onCopied={() => void book.refetch()}
        />
        <Side
          title={t("orders-buying")}
          rows={buying}
          selling={false}
          bulk={bulk}
          onCopied={() => void book.refetch()}
        />
      </div>
    </div>
  );
}
