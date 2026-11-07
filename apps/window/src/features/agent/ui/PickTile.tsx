import type { Good, RivenAppraisal, RivenPick } from "@median/ipc";
import { Check, CircleCheckBig, Copy, ExternalLink, X } from "lucide-react";
import { useState } from "react";
import { Action, Mark } from "@/features/orders/ui/OrderTile";
import { RivenHeading } from "@/features/rivens/ui/RivenHeading";
import { SellerMark } from "@/features/rivens/ui/SellerMark";
import { StatRow } from "@/features/rivens/ui/StatRow";
import { useLocale } from "@/shared/data/app-state";
import { openAuction, useDismissRivenPick, useRecordRivenBuy } from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { formatCount } from "@/shared/lib/format";
import { Money } from "@/shared/ui/money";
import { PriceRow } from "./PriceRow";

/**
 * A riven auction the agent picked: the riven, its buyout against its expected price, the seller,
 * and the first offer to send.
 */
export function PickTile({
  pick,
  weapon,
  name,
  appraisal,
}: {
  pick: RivenPick;
  weapon: Good | undefined;
  name: string | null | undefined;
  appraisal: RivenAppraisal | null | undefined;
}) {
  const t = useT();
  const locale = useLocale();
  const buy = useRecordRivenBuy();
  const dismiss = useDismissRivenPick();
  const [buying, setBuying] = useState(false);
  const [copied, setCopied] = useState(false);
  const number = (value: number) => formatCount(locale, value);
  return (
    <article className="flex flex-col gap-3 rounded-panel border border-line bg-surface p-3">
      <RivenHeading
        riven={pick.riven}
        rank={pick.rank}
        weapon={weapon}
        name={name ?? pick.name}
        appraisal={appraisal}
      />
      {appraisal && (
        <div className="flex flex-col gap-1">
          {appraisal.stats.map((stat) => (
            <StatRow key={`${stat.tag}${stat.isCurse}`} stat={stat} />
          ))}
        </div>
      )}
      {buying ? (
        <PriceRow
          label={t("pick-paid")}
          start={pick.offer}
          onDone={(price) => {
            setBuying(false);
            if (price !== null) buy.mutate({ id: pick.id, price });
          }}
        />
      ) : (
        <div className="flex items-center gap-3">
          <Money
            currency="platinum"
            amount={pick.buyout}
            className="font-semibold text-ink text-title"
          />
          <span className="text-muted text-small">{t("pick-reason", { reason: pick.reason })}</span>
          <span className="ml-auto flex">
            <Action hint={t("pick-open")} onClick={() => openAuction(pick.id)}>
              <ExternalLink />
            </Action>
            <Action
              hint={t("item-copy")}
              onClick={() => {
                void navigator.clipboard.writeText(pick.whisper);
                setCopied(true);
              }}
            >
              {copied ? <Check /> : <Copy />}
            </Action>
            <Action hint={t("pick-bought")} onClick={() => setBuying(true)}>
              <CircleCheckBig />
            </Action>
            <Action hint={t("pick-dismiss")} onClick={() => dismiss.mutate(pick.id)} danger>
              <X />
            </Action>
          </span>
        </div>
      )}
      <div className="grid grid-cols-3 gap-2 border-hairline border-t pt-2.5">
        <Mark label={t("auction-market")}>{number(pick.worth)}</Mark>
        <Mark label={t("pick-offer")}>{number(pick.offer)}</Mark>
        <Mark label={t("pick-seller")}>
          <SellerMark seller={pick.seller} status={pick.status} />
        </Mark>
      </div>
    </article>
  );
}
