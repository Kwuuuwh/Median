import type { OwnOrder } from "@median/ipc";
import { Bot, MoreHorizontal, RefreshCw } from "lucide-react";
import { DropdownMenu } from "radix-ui";
import { useState } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useOwnOrders } from "@/shared/data/market";
import { useOwnAuctions } from "@/shared/data/rivens";
import { useItemSearch } from "@/shared/data/search";
import { useShown } from "@/shared/data/shown";
import {
  useHandOrderToAgent,
  useRefreshOrders,
  useShowOrders,
  useWithdrawOrders,
} from "@/shared/data/trading";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import type { Sorting } from "@/shared/lib/sort";
import { useKept } from "@/shared/lib/use-kept";
import { Button } from "@/shared/ui/button";
import { EmptyState } from "@/shared/ui/empty-state";
import { FacetMenu } from "@/shared/ui/facet-menu";
import { Money } from "@/shared/ui/money";
import { SearchField } from "@/shared/ui/search-field";
import { SortSelect } from "@/shared/ui/sort-select";
import { Tab, TabList, Tabs } from "@/shared/ui/tabs";
import { TileGrid } from "@/shared/ui/tile-grid";
import {
  answer,
  BOOK_FACETS,
  type BookFacet,
  type BookOrder,
  sortBook,
  worthOf,
} from "../model/book";
import { AuctionsTab } from "./AuctionsTab";
import { HandDialog } from "./HandDialog";
import { OrderTile } from "./OrderTile";
import { RecordDialog } from "./RecordDialog";

const ORDERS: readonly BookOrder[] = ["gap", "worth", "price", "quantity", "name"];

/** Class names of a dropdown menu item. */
const MENU_ITEM =
  "flex h-8 cursor-default items-center rounded-control px-2 text-body text-soft outline-none data-[highlighted]:bg-surface data-[highlighted]:text-ink";

/** One side of the account's orders: title, total platinum, a menu to show, hide or delete all of
 * the user's own (non-agent) orders, and the orders as tiles. */
function Side({
  selling,
  orders,
  onRecord,
  onHand,
}: {
  selling: boolean;
  orders: readonly OwnOrder[];
  onRecord: (order: OwnOrder) => void;
  onHand: (order: OwnOrder) => void;
}) {
  const t = useT();
  const showAll = useShowOrders();
  const withdraw = useWithdrawOrders();
  const [isConfirming, setConfirming] = useState(false);
  const total = orders.reduce((sum, order) => sum + worthOf(order), 0);
  const own = orders.filter((order) => !order.byAgent);
  return (
    <section className="flex min-h-0 min-w-0 flex-col">
      <header className="flex shrink-0 items-center gap-3 px-6 py-3">
        <h2 className="font-display font-semibold text-ink text-title">
          {t(selling ? "own-selling" : "own-buying")}
        </h2>
        <span className="text-muted text-small tabular-nums">{orders.length}</span>
        <Money currency="platinum" amount={total} className="text-body text-soft" />
        <DropdownMenu.Root modal={false} onOpenChange={(open) => !open && setConfirming(false)}>
          <DropdownMenu.Trigger
            aria-label={t("trade-more")}
            className="ml-auto inline-flex size-7 items-center justify-center rounded-control text-dim outline-none hover:bg-raised hover:text-ink"
          >
            <MoreHorizontal className="size-4" />
          </DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content
              align="end"
              sideOffset={6}
              className="z-50 min-w-52 rounded-panel border border-line bg-raised p-1 shadow-black/40 shadow-xl"
            >
              <DropdownMenu.Item
                className={MENU_ITEM}
                onSelect={() => showAll.mutate({ selling, visible: true })}
              >
                {t("trade-show-all")}
              </DropdownMenu.Item>
              <DropdownMenu.Item
                className={MENU_ITEM}
                onSelect={() => showAll.mutate({ selling, visible: false })}
              >
                {t("trade-hide-all")}
              </DropdownMenu.Item>
              <DropdownMenu.Separator className="my-1 h-px bg-line" />
              <DropdownMenu.Item
                className={cn(
                  MENU_ITEM,
                  isConfirming && "text-danger data-[highlighted]:text-danger",
                )}
                disabled={own.length === 0 || withdraw.isPending}
                onSelect={(event) => {
                  if (!isConfirming) {
                    event.preventDefault();
                    setConfirming(true);
                    return;
                  }
                  setConfirming(false);
                  withdraw.mutate(own.map((order) => order.id));
                }}
              >
                {t(isConfirming ? "trade-withdraw-all-confirm" : "trade-withdraw-all", {
                  count: own.length,
                })}
              </DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 pb-16">
        {orders.length === 0 ? (
          <EmptyState title={t("deal-orders-none")} />
        ) : (
          <TileGrid least={208}>
            {orders.map((order) => (
              <OrderTile
                key={order.id}
                order={order}
                onRecord={() => onRecord(order)}
                onHand={() => onHand(order)}
              />
            ))}
          </TileGrid>
        )}
      </div>
    </section>
  );
}

/** The account's item orders: sell orders on the left, buy orders on the right, each a tile with
 * market prices that refresh first and inline editing. */
function ItemsTab() {
  const t = useT();
  const locale = useLocale();
  const orders = useOwnOrders();
  const refresh = useRefreshOrders();
  const [sorting, setSorting] = useKept<Sorting<BookOrder>>("orders.by", {
    by: "gap",
    descending: true,
  });
  const [facets, setFacets] = useKept<ReadonlyMap<BookFacet, boolean>>("orders.facets", new Map());
  const [recording, setRecording] = useState<OwnOrder | null>(null);
  const [handing, setHanding] = useState<OwnOrder | "new" | null>(null);
  const handOrder = useHandOrderToAgent();
  const hand = (order: OwnOrder) => {
    if (order.selling) setHanding(order);
    else handOrder.mutate({ id: order.id, platinum: null });
  };
  const search = useItemSearch(
    "orders.search",
    (orders.data ?? []).flatMap((order) => (order.item ? [order.item] : [])),
  );

  useShown(
    (orders.data ?? []).flatMap((order) =>
      order.slug ? [{ slug: order.slug, key: order.variant }] : [],
    ),
  );

  if (orders.isPending) return <EmptyState title={t("deal-orders-asking")} />;

  const shown = sortBook(
    (orders.data ?? []).filter(
      (order) =>
        search.matches(order.item, order.name) &&
        [...facets].every(([facet, wanted]) => answer(facet, order) === wanted),
    ),
    sorting,
    locale,
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-line border-b px-6 py-3">
        <SearchField
          {...search.field}
          placeholder={t("shelf-search")}
          className="w-56 min-w-36 shrink"
        />
        <FacetMenu facets={BOOK_FACETS} values={facets} onChange={setFacets} className="w-44" />
        <SortSelect<BookOrder>
          label={t("shelf-sort")}
          className="w-40"
          value={sorting}
          onChange={setSorting}
          choices={ORDERS.map((order) => ({ value: order, label: t("trade-order", { order }) }))}
        />
        <Button tone="ghost" className="ml-auto" onClick={() => setHanding("new")}>
          <Bot />
          {t("hand-to-agent")}
        </Button>
        <Button
          tone="ghost"
          size="icon"
          aria-label={t("trade-refresh")}
          disabled={refresh.isPending}
          onClick={() => refresh.mutate(undefined)}
        >
          <RefreshCw className={refresh.isPending ? "animate-spin" : undefined} />
        </Button>
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-2 divide-x divide-line">
        <Side
          selling
          orders={shown.filter((order) => order.selling)}
          onRecord={setRecording}
          onHand={hand}
        />
        <Side
          selling={false}
          orders={shown.filter((order) => !order.selling)}
          onRecord={setRecording}
          onHand={hand}
        />
      </div>
      <RecordDialog order={recording} onClose={() => setRecording(null)} />
      {handing !== null && (
        <HandDialog order={handing === "new" ? null : handing} onClose={() => setHanding(null)} />
      )}
    </div>
  );
}

type OrdersTab = "items" | "rivens";

/** Orders page: the account's item orders on one tab, its riven auctions on the other. */
export function OrdersPage() {
  const t = useT();
  const [tab, setTab] = useKept<OrdersTab>("orders.tab", "items");
  const orders = useOwnOrders().data;
  const auctions = useOwnAuctions().data;
  return (
    <Tabs
      value={tab}
      onValueChange={(value) => setTab(value as OrdersTab)}
      className="flex min-w-0 flex-1 flex-col"
    >
      <div className="flex shrink-0 items-end border-line border-b px-6 pt-3">
        <TabList className="shrink-0 border-b-0">
          <Tab value="items" count={orders?.length}>
            {t("orders-items")}
          </Tab>
          <Tab value="rivens" count={auctions?.length}>
            {t("orders-rivens")}
          </Tab>
        </TabList>
      </div>
      {tab === "items" ? <ItemsTab /> : <AuctionsTab />}
    </Tabs>
  );
}
