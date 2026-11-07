import type { Good, OwnAuction, RivenAppraisal } from "@median/ipc";
import { Check, Eye, EyeOff, Pencil, Trash2, X } from "lucide-react";
import { type KeyboardEvent, useState } from "react";
import { RivenHeading } from "@/features/rivens/ui/RivenHeading";
import { StatRow } from "@/features/rivens/ui/StatRow";
import { useLocale } from "@/shared/data/app-state";
import { useChangeAuction, useCloseAuction, useRivenChoices } from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { Input } from "@/shared/ui/input";
import { Money } from "@/shared/ui/money";
import { wholeNumber } from "@/shared/ui/number-field";
import { Action, Mark } from "./OrderTile";

/** Highest auction price the editor accepts, in platinum. */
const MOST = 100_000;

/** An auction's starting price and buyout. */
type Prices = { starting: number; buyout: number | null };

/**
 * Inline auction price editor: Enter confirms valid prices, Escape cancels with null; a direct
 * sale uses one price for both.
 */
function EditRow({
  auction,
  onDone,
}: {
  auction: OwnAuction;
  onDone: (prices: Prices | null) => void;
}) {
  const t = useT();
  const [starting, setStarting] = useState(String(auction.startingPrice));
  const [buyout, setBuyout] = useState(
    auction.buyoutPrice === null ? "" : String(auction.buyoutPrice),
  );
  const start = wholeNumber(starting, 1, MOST);
  const end = buyout === "" ? null : wholeNumber(buyout, start ?? 1, MOST);
  const keep = () => {
    if (start === null) return;
    if (auction.isDirectSell) onDone({ starting: start, buyout: start });
    else if (buyout === "" || end !== null) onDone({ starting: start, buyout: end });
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
        aria-label={t(auction.isDirectSell ? "trade-price" : "riven-starting")}
        aria-invalid={start === null}
        value={starting}
        onChange={(event) => setStarting(event.target.value)}
        onKeyDown={keys}
        className="w-20"
      />
      {!auction.isDirectSell && (
        <Input
          type="number"
          min={1}
          aria-label={t("riven-buyout")}
          aria-invalid={buyout !== "" && end === null}
          value={buyout}
          onChange={(event) => setBuyout(event.target.value)}
          onKeyDown={keys}
          className="w-20"
        />
      )}
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

/** Tile of one of the account's riven auctions. */
export function AuctionTile({
  auction,
  weapon,
  name,
  appraisal,
  onOpen,
}: {
  auction: OwnAuction;
  weapon: Good | undefined;
  name: string | null | undefined;
  appraisal: RivenAppraisal | null | undefined;
  onOpen: () => void;
}) {
  const t = useT();
  const locale = useLocale();
  const change = useChangeAuction();
  const close = useCloseAuction();
  const [editing, setEditing] = useState(false);
  const isBusy = change.isPending || close.isPending;
  const { riven } = auction;
  const band = useRivenChoices(riven).data?.sell ?? null;
  const asked = auction.buyoutPrice ?? auction.startingPrice;
  const gap = band?.middle == null ? null : Math.round(asked - band.middle);
  const number = (value: number | null | undefined) =>
    value == null ? "—" : formatCount(locale, Math.round(value));
  const amend = (prices: Prices, visible: boolean) =>
    change.mutate({ id: auction.id, ...prices, visible });
  return (
    <article
      className={cn(
        "flex flex-col gap-3 rounded-panel border border-line bg-surface p-3 transition-colors hover:border-dim",
        !auction.visible && "opacity-60",
      )}
    >
      {riven ? (
        <button type="button" onClick={onOpen} className="flex flex-col gap-2.5 text-left">
          <RivenHeading
            riven={riven}
            rank={auction.rank}
            weapon={weapon}
            name={name ?? auction.name}
            appraisal={appraisal}
          />
          {appraisal && (
            <span className="flex w-full flex-col gap-1">
              {appraisal.stats.map((stat) => (
                <StatRow key={`${stat.tag}${stat.isCurse}`} stat={stat} />
              ))}
            </span>
          )}
        </button>
      ) : (
        <span className="grid gap-0.5 text-body leading-tight">
          <span className="font-medium text-soft">{weapon?.name ?? auction.name}</span>
          <span className="text-caption text-dim">{t("riven-unvalued")}</span>
        </span>
      )}
      {editing ? (
        <EditRow
          auction={auction}
          onDone={(prices) => {
            setEditing(false);
            if (
              prices &&
              (prices.starting !== auction.startingPrice || prices.buyout !== auction.buyoutPrice)
            )
              amend(prices, auction.visible);
          }}
        />
      ) : (
        <div className="flex items-center gap-3">
          <Money currency="platinum" amount={asked} className="font-semibold text-ink text-title" />
          {!auction.isDirectSell && (
            <span className="text-muted text-small tabular-nums">
              {auction.buyoutPrice === null
                ? t("auction-bids-only")
                : t("auction-from", { starting: number(auction.startingPrice) })}
            </span>
          )}
          <span className="ml-auto flex">
            <Action hint={t("trade-edit")} onClick={() => setEditing(true)} disabled={isBusy}>
              <Pencil />
            </Action>
            <Action
              hint={t(auction.visible ? "trade-hide" : "trade-show")}
              onClick={() =>
                amend(
                  { starting: auction.startingPrice, buyout: auction.buyoutPrice },
                  !auction.visible,
                )
              }
              disabled={isBusy}
            >
              {auction.visible ? <EyeOff /> : <Eye />}
            </Action>
            <Action
              hint={t("trade-withdraw")}
              onClick={() => close.mutate(auction.id)}
              disabled={isBusy}
              danger
            >
              <Trash2 />
            </Action>
          </span>
        </div>
      )}
      <div className="grid grid-cols-3 gap-2 border-hairline border-t pt-2.5">
        <Mark label={t("auction-market")}>
          {number(band?.middle)}
          {gap !== null && gap !== 0 && (
            <span className="ml-1 text-muted">
              ({gap > 0 ? "+" : "−"}
              {formatCount(locale, Math.abs(gap))})
            </span>
          )}
        </Mark>
        <Mark label={t("auction-band")}>
          {band ? `${number(band.low)}–${number(band.high)}` : "—"}
        </Mark>
        {auction.isDirectSell ? (
          <Mark label={t("auction-lots")}>{band ? formatCount(locale, band.lots) : "—"}</Mark>
        ) : (
          <Mark label={t("riven-top-bid")}>{number(auction.topBid)}</Mark>
        )}
      </div>
    </article>
  );
}
