import type { Card } from "@median/ipc";
import { Inbox, X } from "lucide-react";
import { useState } from "react";
import { useCard, useGoods, useSets, useTaxonomy } from "@/shared/data/catalog";
import { useHoldings } from "@/shared/data/holdings";
import { usePairs, useQuotes } from "@/shared/data/market";
import { useOpened } from "@/shared/data/shown";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { useKept } from "@/shared/lib/use-kept";
import { useOverflowsRight } from "@/shared/lib/use-overflows-right";
import { DialogClose, DialogDescription, DialogTitle } from "@/shared/ui/dialog";
import { EmptyLine } from "@/shared/ui/empty-state";
import { Money } from "@/shared/ui/money";
import { Tab, TabList, TabPanel, Tabs } from "@/shared/ui/tabs";
import { compositionOf, partName } from "../../model/composition";
import type { RankPick } from "../../model/orders";
import { sourceCounts } from "../../model/sources";
import { FavouriteStar } from "../FavouriteStar";
import { HeldMark } from "../HeldMark";
import { ItemThumb } from "../ItemThumb";
import { ItemContents } from "./ItemContents";
import { ItemMarket } from "./ItemMarket";
import { ItemOrders } from "./ItemOrders";
import { ItemSources } from "./ItemSources";
import { SetSources } from "./SetSources";
import { TradeBar } from "./TradeBar";

/** Whether a separate blueprint builds the item, so its drop sources are the blueprint's. */
function builds(card: Card, item: string): boolean {
  return card.recipes.some((recipe) => recipe.blueprint !== item);
}

type Pane = "contents" | "sources" | "orders";

/**
 * Item card: its set part by part, the market strip, and tabs for contents, sources and orders;
 * a tab without content is not shown.
 */
export function ItemSheet({
  item,
  variant,
  onOpen,
}: {
  item: string;
  /** Variant of the held copy the card was opened from; null when not opened from one. */
  variant: string | null;
  onOpen: (item: string) => void;
}) {
  const t = useT();
  const sets = useSets().data ?? [];
  const own = useCard(item).data;
  const builder = own?.recipes.find((recipe) => recipe.blueprint !== item) ?? null;
  const { set, members, fromRecipe } = compositionOf(item, sets, builder, own?.good.name);
  const [focus, setFocus] = useState(item);
  const [pane, setPane] = useState<Pane | null>(null);
  const [strip, setStrip] = useState<HTMLElement | null>(null);
  const stripOverflows = useOverflowsRight(strip);
  const [rank, setRank] = useKept<RankPick>("orders.rank", "zero");
  const goods = new Map(
    (useGoods([...members].sort()).data ?? []).map((good) => [good.item, good]),
  );
  const quotes = useQuotes(members).data;
  const pieces = useHoldings().data?.pieces ?? [];
  const kinds = new Map((useTaxonomy().data?.kinds ?? []).map((kind) => [kind.slug, kind.name]));
  const card = useCard(focus).data;
  const pair = usePairs([focus]).data?.get(focus) ?? null;
  useOpened(pair);
  const held = (key: string) =>
    pieces.filter((piece) => piece.item === key).reduce((sum, piece) => sum + piece.count, 0);

  const root = set?.built ?? item;
  const rootGood = goods.get(root) ?? (root === focus ? card?.good : undefined);
  const rootName = rootGood?.name ?? set?.name ?? item;
  const label = (key: string) =>
    key === set?.built
      ? fromRecipe
        ? rootName
        : t("item-whole")
      : partName(goods.get(key)?.name ?? key, rootName);
  const whole = set !== null && focus === set.built;
  const parts = members
    .filter((key) => key !== set?.built)
    .map((key) => ({ item: key, name: label(key), image: goods.get(key)?.image ?? null }));
  const kind = rootGood ? kinds.get(rootGood.kind) : undefined;
  const panes: Pane[] = [
    ...(pair ? (["orders"] as const) : []),
    ...((card?.inside.length ?? 0) > 0 ? (["contents"] as const) : []),
    ...(whole || (card && (sourceCounts(card).size > 0 || builds(card, focus)))
      ? (["sources"] as const)
      : []),
  ];
  const current = pane && panes.includes(pane) ? pane : panes[0];
  const copies = pieces.filter(
    (piece) =>
      piece.item === focus &&
      piece.count > 0 &&
      (focus !== item || variant === null || piece.variant === variant),
  );
  const sellable = whole
    ? Math.min(...parts.map((part) => held(part.item)))
    : copies.reduce((sum, piece) => sum + piece.count, 0);
  const tradable = (whole && !fromRecipe) || (goods.get(focus) ?? card?.good)?.tradable === true;

  return (
    <>
      <header className="flex shrink-0 items-start gap-3 border-hairline border-b p-4">
        <ItemThumb image={rootGood?.image ?? null} mastered={rootGood?.mastered === true} />
        <div className="grid min-w-0 flex-1 gap-0.5 pt-0.5">
          <div className="flex flex-wrap items-center gap-x-2.5">
            <DialogTitle className="text-heading text-ink">{rootName}</DialogTitle>
            <HeldMark count={held(root)} />
          </div>
          <DialogDescription>
            {[kind, rootGood?.vaulted && t("relic-vaulted")].filter(Boolean).join(" · ")}
          </DialogDescription>
        </div>
        <FavouriteStar item={root} />
        <DialogClose
          aria-label={t("card-close")}
          className="inline-flex size-7 items-center justify-center rounded-control text-muted hover:bg-raised hover:text-ink"
        >
          <X className="size-4" />
        </DialogClose>
      </header>
      {members.length > 1 && (
        <nav
          ref={setStrip}
          className={cn(
            "flex shrink-0 gap-2 overflow-x-auto border-hairline border-b px-4 py-3",
            stripOverflows && "[mask-image:linear-gradient(to_right,black_85%,transparent)]",
          )}
        >
          {members.map((key) => (
            <button
              key={key}
              type="button"
              onClick={() => setFocus(key)}
              aria-pressed={focus === key}
              className={cn(
                "flex shrink-0 items-center gap-2 rounded-control border px-2.5 py-1.5 text-left transition-colors",
                focus === key ? "border-accent/60 bg-raised" : "border-line hover:border-dim",
              )}
            >
              <ItemThumb image={goods.get(key)?.image ?? null} size="sm" />
              <span className="grid">
                <span className="flex items-center gap-1.5 text-body text-ink">
                  {label(key)}
                  <HeldMark count={held(key)} />
                </span>
                <Money
                  currency="platinum"
                  amount={quotes?.get(key) ?? null}
                  className="text-small text-soft"
                />
              </span>
            </button>
          ))}
        </nav>
      )}
      <div className="flex min-h-0 flex-col overflow-y-auto">
        <ItemMarket item={focus} rank={rank} onRank={setRank} />
        {current ? (
          <Tabs value={current} onValueChange={(value) => setPane(value as Pane)}>
            <TabList className="sticky top-0 z-10 bg-surface px-4">
              {panes.map((each) => (
                <Tab key={each} value={each}>
                  {t(
                    each === "sources"
                      ? "item-sources"
                      : each === "orders"
                        ? "orders-title"
                        : "card-inside",
                  )}
                </Tab>
              ))}
            </TabList>
            <div className="p-4">
              <TabPanel value="contents">
                <ItemContents item={focus} onOpen={onOpen} />
              </TabPanel>
              <TabPanel value="sources">
                {whole ? (
                  <SetSources parts={parts} onOpen={setFocus} />
                ) : (
                  <ItemSources item={focus} onOpen={onOpen} />
                )}
              </TabPanel>
              <TabPanel value="orders">
                <ItemOrders item={focus} rank={rank} />
              </TabPanel>
            </div>
          </Tabs>
        ) : (
          card !== undefined && (
            <EmptyLine icon={Inbox} className="px-4 py-3">
              {t("card-empty")}
            </EmptyLine>
          )
        )}
      </div>
      {tradable && current !== "sources" && (
        <footer className="grid shrink-0 grid-cols-2 divide-x divide-line border-line border-t bg-surface">
          {sellable > 0 ? (
            <TradeBar
              key={`sell:${focus}`}
              item={focus}
              selling
              copies={whole ? [] : copies}
              held={sellable}
              price={quotes?.get(focus) ?? null}
            />
          ) : (
            <span />
          )}
          <TradeBar
            key={`buy:${focus}`}
            item={focus}
            selling={false}
            copies={whole ? [] : copies}
            held={sellable}
            price={quotes?.get(focus) ?? null}
          />
        </footer>
      )}
    </>
  );
}
