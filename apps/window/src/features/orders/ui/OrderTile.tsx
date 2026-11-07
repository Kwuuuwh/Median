import type { OwnOrder } from "@median/ipc";
import { Bot, Check, CircleCheckBig, Eye, EyeOff, Pencil, Plus, Trash2, X } from "lucide-react";
import { type KeyboardEvent, type ReactNode, useState } from "react";
import { openItem } from "@/features/items/model/open";
import { useLocale } from "@/shared/data/app-state";
import { useAmendOrder, useWithdrawOrders } from "@/shared/data/trading";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { Input } from "@/shared/ui/input";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Money } from "@/shared/ui/money";
import { wholeNumber } from "@/shared/ui/number-field";
import { Tooltip } from "@/shared/ui/tooltip";
import { gapOf } from "../model/book";

/** Highest price or quantity the editor accepts. */
const MOST = 100_000;

/** Rank in a variant key such as `rank=5`; null when the key has none. */
function rankOf(variant: string): string | null {
  return /rank=(\d+)/.exec(variant)?.[1] ?? null;
}

/** One value of the tile's market line: label and value. */
export function Mark({ label, children }: { label: string; children: ReactNode }) {
  return (
    <span className="grid min-w-0 gap-0.5">
      <span className="truncate text-caption text-dim">{label}</span>
      <span className="flex min-w-0 text-small text-soft tabular-nums">{children}</span>
    </span>
  );
}

/** Small square icon button with its label as a tooltip and accessible name. */
export function Action({
  hint,
  onClick,
  danger = false,
  disabled = false,
  children,
}: {
  hint: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  children: ReactNode;
}) {
  return (
    <Tooltip hint={hint}>
      <button
        type="button"
        aria-label={hint}
        onClick={onClick}
        disabled={disabled}
        className={cn(
          "inline-flex size-7 items-center justify-center rounded-control text-dim transition-colors hover:bg-raised disabled:pointer-events-none disabled:opacity-40 [&_svg]:size-4",
          danger ? "hover:text-danger" : "hover:text-ink",
        )}
      >
        {children}
      </button>
    </Tooltip>
  );
}

/**
 * Inline price and quantity editor: Enter or the check button confirms valid values, Escape or
 * the cross cancels with null.
 */
function EditRow({
  order,
  onDone,
}: {
  order: OwnOrder;
  onDone: (change: { platinum: number; quantity: number } | null) => void;
}) {
  const t = useT();
  const [platinum, setPlatinum] = useState(String(order.platinum));
  const [quantity, setQuantity] = useState(String(order.quantity));
  const price = wholeNumber(platinum, 1, MOST);
  const count = wholeNumber(quantity, 1, MOST);
  const keep = () => {
    if (price !== null && count !== null) onDone({ platinum: price, quantity: count });
  };
  const keys = (event: KeyboardEvent) => {
    if (event.key === "Enter") keep();
    if (event.key === "Escape") onDone(null);
  };
  return (
    <div className="flex items-center gap-2">
      <Input
        type="number"
        min={1}
        autoFocus
        aria-label={t("trade-price")}
        aria-invalid={price === null}
        value={platinum}
        onChange={(event) => setPlatinum(event.target.value)}
        onKeyDown={keys}
        className="w-20"
      />
      <span className="text-dim">×</span>
      <Input
        type="number"
        min={1}
        aria-label={t("trade-quantity")}
        aria-invalid={count === null}
        value={quantity}
        onChange={(event) => setQuantity(event.target.value)}
        onKeyDown={keys}
        className="w-16"
      />
      <span className="ml-auto flex">
        <Action hint={t("trade-keep")} onClick={keep}>
          <Check />
        </Action>
        <Action hint={t("trade-cancel")} onClick={() => onDone(null)}>
          <X />
        </Action>
      </span>
    </div>
  );
}

/** Tile of one of the account's orders: the item, its price and quantity, market prices, and
 * actions: edit, add one, record as fulfilled, pass to the agent, delete, show or hide. Orders the
 * agent placed get no actions, because only the agent changes them. */
export function OrderTile({
  order,
  onRecord,
  onHand,
}: {
  order: OwnOrder;
  /** Records the user's order as fulfilled; unused for orders the agent placed. */
  onRecord?: () => void;
  onHand?: () => void;
}) {
  const t = useT();
  const locale = useLocale();
  const amend = useAmendOrder();
  const withdraw = useWithdrawOrders();
  const [editing, setEditing] = useState(false);
  const isBusy = amend.isPending || withdraw.isPending;
  const gap = gapOf(order);
  const rank = rankOf(order.variant);
  const change = (platinum: number | null, quantity: number | null, visible: boolean | null) =>
    amend.mutate({ id: order.id, amend: { platinum, quantity, visible } });
  return (
    <article
      className={cn(
        "group flex flex-col gap-3 rounded-panel border border-line bg-surface p-3 transition-colors hover:border-dim",
        !order.visible && "opacity-60",
      )}
    >
      <header className="flex min-w-0 items-start gap-2.5">
        <button
          type="button"
          disabled={!order.item}
          onClick={() => openItem(order.item)}
          className="flex min-w-0 flex-1 items-center gap-2.5 text-left"
        >
          <ItemPicture
            image={order.image}
            className="size-11 shrink-0 rounded-control bg-sunken p-0.5"
          />
          <span className="grid min-w-0 gap-0.5">
            <span className="line-clamp-2 text-body text-ink leading-snug hover:text-accent">
              {order.name}
            </span>
            <span className="flex items-center gap-1.5 text-muted text-small tabular-nums">
              {rank !== null && (
                <span>{order.maxRank !== null ? `${rank}/${order.maxRank}` : rank}</span>
              )}
              {order.perTrade > 1 && <span className="text-plat">×{order.perTrade}</span>}
              {order.byAgent && (
                <Tooltip hint={t("trade-by-agent")}>
                  <Bot className="size-3.5 text-dim" />
                </Tooltip>
              )}
            </span>
          </span>
        </button>
        {!order.byAgent && (
          <Action
            hint={t(order.visible ? "trade-hide" : "trade-show")}
            onClick={() => change(null, null, !order.visible)}
            disabled={isBusy}
          >
            {order.visible ? <Eye /> : <EyeOff />}
          </Action>
        )}
      </header>
      {editing ? (
        <EditRow
          order={order}
          onDone={(edited) => {
            setEditing(false);
            if (
              edited &&
              (edited.platinum !== order.platinum || edited.quantity !== order.quantity)
            )
              change(edited.platinum, edited.quantity, null);
          }}
        />
      ) : (
        <div className="flex items-center gap-3">
          <Money
            currency="platinum"
            amount={order.platinum}
            className="font-semibold text-ink text-title"
          />
          <span className="text-body text-soft tabular-nums">×{order.quantity}</span>
          {!order.byAgent && (
            <span className="ml-auto flex">
              <Action hint={t("trade-edit")} onClick={() => setEditing(true)} disabled={isBusy}>
                <Pencil />
              </Action>
              <Action
                hint={t("trade-add-one")}
                onClick={() => change(null, order.quantity + 1, null)}
                disabled={isBusy}
              >
                <Plus />
              </Action>
              <Action hint={t("trade-fulfil")} onClick={() => onRecord?.()} disabled={isBusy}>
                <CircleCheckBig />
              </Action>
              <Action hint={t("hand-to-agent")} onClick={() => onHand?.()} disabled={isBusy}>
                <Bot />
              </Action>
              <Action
                hint={t("trade-withdraw")}
                onClick={() => withdraw.mutate([order.id])}
                disabled={isBusy}
                danger
              >
                <Trash2 />
              </Action>
            </span>
          )}
        </div>
      )}
      <div className="grid grid-cols-3 gap-2 border-hairline border-t pt-2.5">
        <Mark label={t("trade-reference")}>
          {order.reference === null ? "—" : formatCount(locale, order.reference)}
          {gap !== null && gap !== 0 && (
            <span className="ml-1 text-muted">
              ({gap > 0 ? "+" : "−"}
              {Math.abs(gap)})
            </span>
          )}
        </Mark>
        {order.selling ? (
          <Mark label={t("trade-median")}>{order.ours ?? "—"}</Mark>
        ) : (
          <Mark label={t("trade-resale")}>{order.resale ?? "—"}</Mark>
        )}
        <Mark label={t(order.selling ? "trade-held" : "trade-most")}>
          {(order.selling ? order.owned : order.most) ?? "—"}
        </Mark>
      </div>
    </article>
  );
}
