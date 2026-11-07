import type { OwnOrder } from "@median/ipc";
import { Lock, Tag } from "lucide-react";
import { useState } from "react";
import { openItem } from "@/features/items/model/open";
import { TradeBar } from "@/features/items/ui/card/TradeBar";
import { ListedBadge } from "@/features/items/ui/ListedBadge";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Button } from "@/shared/ui/button";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Money } from "@/shared/ui/money";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";
import { Segmented } from "@/shared/ui/segmented";
import { Tooltip } from "@/shared/ui/tooltip";
import { type Kit, type KitPart, missingCost, missingOf } from "../model/kits";

/**
 * One set part: picture with held count and sell order badge, short name and price; a part held
 * below the needed count gets a dashed border.
 */
function PartSlot({ part, listing }: { part: KitPart; listing?: OwnOrder }) {
  const missing = part.held < part.need;
  const none = part.held === 0;
  return (
    <button
      type="button"
      onClick={() => openItem(part.item)}
      className="group/part grid w-16 justify-items-center gap-1 text-center"
    >
      <span
        className={cn(
          "relative block size-14 rounded-control border transition-colors group-hover/part:border-dim",
          missing ? "border-line border-dashed bg-transparent" : "border-line bg-sunken",
        )}
      >
        <ItemPicture
          image={part.image}
          className={cn("size-full p-1", none && "opacity-30 grayscale")}
        />
        {listing && (
          <span className="absolute -top-1.5 left-1/2 -translate-x-1/2">
            <ListedBadge listing={listing} counted compact />
          </span>
        )}
        {(part.held > 1 || part.need > 1) && (
          <span className="absolute -right-1 -bottom-1 rounded-sm bg-canvas/90 px-1 font-medium text-[0.7rem] text-ink tabular-nums leading-tight">
            {part.need > 1 ? `${part.held}/${part.need}` : `×${part.held}`}
          </span>
        )}
      </span>
      <span
        className={cn(
          "w-full truncate text-caption leading-tight",
          missing ? "text-dim" : "text-soft",
        )}
      >
        {part.name}
      </span>
      <Money
        currency="platinum"
        amount={part.price}
        className={cn("text-caption", missing ? "text-soft" : "text-muted")}
      />
    </button>
  );
}

/** Popover on the set tile to place a sell order for the complete set or any held part. */
function SellPopover({ kit }: { kit: Kit }) {
  const t = useT();
  const choices = [
    ...(kit.whole > 0
      ? [{ item: kit.item, name: t("item-whole"), held: kit.whole, price: kit.price }]
      : []),
    ...kit.parts
      .filter((part) => part.held > 0)
      .map((part) => ({ item: part.item, name: part.name, held: part.held, price: part.price })),
  ];
  const [picked, setPicked] = useState(choices[0]?.item ?? "");
  const chosen = choices.find((choice) => choice.item === picked) ?? choices[0];
  if (!chosen) return null;
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button size="sm" className="ml-auto">
          <Tag />
          {t("sell-place")}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="grid w-[30rem] gap-2 p-0 pt-3">
        <Segmented
          label={t("sell-place")}
          className="mx-4 flex-wrap justify-self-start"
          value={chosen.item}
          onChange={setPicked}
          segments={choices.map((choice) => ({ value: choice.item, label: choice.name }))}
        />
        <TradeBar
          key={chosen.item}
          item={chosen.item}
          selling
          copies={[]}
          held={chosen.held}
          price={chosen.price}
        />
      </PopoverContent>
    </Popover>
  );
}

/**
 * Tile of a set the account holds any part of: complete sets, every part, the missing parts and
 * their market cost, and a sell order button.
 */
export function KitTile({
  kit,
  listingFor,
}: {
  kit: Kit;
  /** The account's open sell order for an item or the set, if any. */
  listingFor: (item: string, slug: string | null) => OwnOrder | undefined;
}) {
  const t = useT();
  const missing = missingOf(kit);
  const cost = missingCost(kit);
  const held = kit.parts.length - missing.length;
  const listed = listingFor(kit.item, kit.slug);
  return (
    <article className="grid gap-3 rounded-panel border border-line bg-surface p-3">
      <header className="flex min-w-0 items-center gap-3">
        <button
          type="button"
          onClick={() => openItem(kit.item)}
          className="flex min-w-0 flex-1 items-center gap-3 text-left"
        >
          <span className="relative shrink-0">
            <ItemPicture image={kit.image} className="size-11 rounded-control bg-sunken p-0.5" />
            {listed && (
              <span className="absolute -bottom-1.5 left-1/2 -translate-x-1/2">
                <ListedBadge listing={listed} counted compact />
              </span>
            )}
          </span>
          <span className="grid min-w-0 gap-0.5">
            <span className="truncate font-medium text-body text-ink hover:text-accent">
              {kit.name}
            </span>
            <span className="flex items-center gap-2 text-muted text-small tabular-nums">
              {t("kit-parts", { held, total: kit.parts.length })}
              {kit.vaulted && (
                <Tooltip hint={t("relic-vaulted")}>
                  <Lock className="size-3.5" />
                </Tooltip>
              )}
            </span>
          </span>
        </button>
        <span className="grid justify-items-end gap-0.5">
          <Money currency="platinum" amount={kit.price} className="font-semibold text-ink" />
          {kit.whole > 0 && (
            <span className="rounded-sm bg-accent/15 px-1.5 text-accent text-small tabular-nums">
              {t("kit-whole", { count: kit.whole })}
            </span>
          )}
        </span>
      </header>
      <div className="flex flex-wrap gap-2">
        {kit.parts.map((part) => (
          <PartSlot key={part.item} part={part} listing={listingFor(part.item, null)} />
        ))}
      </div>
      <footer className="flex items-center gap-3 border-hairline border-t pt-2.5 text-small">
        {missing.length > 0 && (
          <span className="flex items-center gap-1.5 text-muted">
            {t("kit-missing", { count: missing.length })}
            <span className="text-dim">·</span>
            {t("kit-buy-for")}
            <Money currency="platinum" amount={cost} className="text-ink" />
          </span>
        )}
        <SellPopover kit={kit} />
      </footer>
    </article>
  );
}
