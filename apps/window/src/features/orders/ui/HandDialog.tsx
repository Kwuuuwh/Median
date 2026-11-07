import type { OwnOrder } from "@median/ipc";
import { type FormEvent, useState } from "react";
import {
  freeCount,
  type HeldLine,
  type InventoryTab,
  SET_GROUP,
} from "@/features/inventory/model/lines";
import { useInventory } from "@/features/inventory/ui/useInventory";
import { useItemSearch } from "@/shared/data/search";
import { useHandOrderToAgent, useHandToAgent } from "@/shared/data/trading";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { ItemPicture } from "@/shared/ui/item-picture";
import { SearchField } from "@/shared/ui/search-field";

const TABS: readonly InventoryTab[] = ["parts", "mods", "arcanes", "other"];

/** Maximum number of tiles the picker shows. */
const SHOWN = 24;

/** Tradable inventory lines listed on warframe.market with free copies; complete sets excluded. */
function handable(lines: readonly HeldLine[]): HeldLine[] {
  return lines.filter(
    (line) =>
      line.group !== SET_GROUP && line.tradable && line.pair !== null && freeCount(line) > 0,
  );
}

/** A selectable inventory line: picture, name, rank and free copies. */
function Pick({ line, picked, onPick }: { line: HeldLine; picked: boolean; onPick: () => void }) {
  return (
    <button
      type="button"
      onClick={onPick}
      className={cn(
        "flex min-w-0 items-center gap-2 rounded-control border p-1.5 text-left transition-colors",
        picked ? "border-accent bg-raised" : "border-line hover:border-dim",
      )}
    >
      <ItemPicture image={line.image} className="size-8 shrink-0 rounded-control bg-sunken" />
      <span className="grid min-w-0 gap-0.5">
        <span className="truncate text-body text-ink">{line.name}</span>
        <span className="flex gap-1.5 text-muted text-small tabular-nums">
          {line.rank !== null && <span>{line.rank}</span>}
          <span>×{freeCount(line)}</span>
        </span>
      </span>
    </button>
  );
}

/** Inventory line picker with name search, highest price first, at most `SHOWN` tiles. */
function Picker({ picked, onPick }: { picked: HeldLine | null; onPick: (line: HeldLine) => void }) {
  const t = useT();
  const inventory = useInventory();
  const lines = handable(TABS.flatMap((tab) => inventory.lines(tab)));
  const search = useItemSearch(
    "orders.hand.search",
    lines.map((line) => line.item),
  );
  const shown = lines
    .filter((line) => search.matches(line.item, line.name))
    .sort((a, b) => (b.platinum ?? 0) - (a.platinum ?? 0))
    .slice(0, SHOWN);
  return (
    <div className="grid gap-2">
      <SearchField {...search.field} placeholder={t("shelf-search")} className="w-full" />
      <div className="grid max-h-72 grid-cols-2 gap-1.5 overflow-y-auto">
        {shown.map((line) => (
          <Pick
            key={`${line.item}|${line.variant}`}
            line={line}
            picked={picked?.item === line.item && picked.variant === line.variant}
            onPick={() => onPick(line)}
          />
        ))}
      </div>
    </div>
  );
}

/** Dialog that passes copies to the agent: an inventory line picked here, or the user's sell
 * `order` with the cost of one copy. */
export function HandDialog({ order, onClose }: { order: OwnOrder | null; onClose: () => void }) {
  const t = useT();
  const hand = useHandToAgent();
  const handOrder = useHandOrderToAgent();
  const [picked, setPicked] = useState<HeldLine | null>(null);
  const [quantity, setQuantity] = useState("1");
  const [platinum, setPlatinum] = useState("");
  const count = order ? order.quantity : Number.parseInt(quantity, 10);
  const most = order ? order.quantity : picked ? freeCount(picked) : 0;
  const each = Number.parseInt(platinum, 10);
  const ready =
    (order !== null || picked !== null) &&
    Number.isFinite(count) &&
    count >= 1 &&
    count <= most &&
    Number.isFinite(each) &&
    each >= 1;
  const isPending = hand.isPending || handOrder.isPending;

  function submit(event: FormEvent): void {
    event.preventDefault();
    if (!ready) return;
    if (order) handOrder.mutate({ id: order.id, platinum: each }, { onSuccess: onClose });
    else if (picked)
      hand.mutate(
        { item: picked.item, variant: picked.variant, quantity: count, platinum: each },
        { onSuccess: onClose },
      );
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent className={order ? "w-80" : "w-[30rem]"}>
        <DialogTitle>{t("hand-to-agent-title")}</DialogTitle>
        <DialogDescription className="mt-1">
          {order ? `${order.name} · ×${order.quantity}` : t("hand-to-agent-note")}
        </DialogDescription>
        <form className="mt-4 grid gap-3" onSubmit={submit}>
          {!order && <Picker picked={picked} onPick={setPicked} />}
          <div className="flex items-end gap-2">
            {!order && (
              <label className="grid w-24 gap-1 text-muted text-small">
                {t("hand-to-agent-quantity")}
                <Input
                  type="number"
                  min={1}
                  max={most || undefined}
                  value={quantity}
                  onChange={(event) => setQuantity(event.target.value)}
                />
              </label>
            )}
            <label className="grid min-w-0 flex-1 gap-1 text-muted text-small">
              {t("hand-to-agent-price")}
              <Input
                type="number"
                min={1}
                autoFocus={order !== null}
                value={platinum}
                onChange={(event) => setPlatinum(event.target.value)}
              />
            </label>
            <Button tone="primary" type="submit" disabled={!ready || isPending}>
              {t("hand-to-agent-save")}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
