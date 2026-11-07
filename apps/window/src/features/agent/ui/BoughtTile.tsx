import type { Good, RivenAppraisal, RivenBuy } from "@median/ipc";
import { HandCoins } from "lucide-react";
import { useState } from "react";
import { Action, Mark } from "@/features/orders/ui/OrderTile";
import { RivenHeading } from "@/features/rivens/ui/RivenHeading";
import { StatRow } from "@/features/rivens/ui/StatRow";
import { useLocale } from "@/shared/data/app-state";
import { useRecordRivenSale, useRivenChoices } from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { formatCount, formatDay } from "@/shared/lib/format";
import { Money } from "@/shared/ui/money";
import { PriceRow } from "./PriceRow";

/** An unsold riven bought for resale: purchase price against asking prices of similar rivens. */
export function BoughtTile({
  buy,
  weapon,
  name,
  appraisal,
}: {
  buy: RivenBuy;
  weapon: Good | undefined;
  name: string | null | undefined;
  appraisal: RivenAppraisal | null | undefined;
}) {
  const t = useT();
  const locale = useLocale();
  const sell = useRecordRivenSale();
  const [selling, setSelling] = useState(false);
  const band = useRivenChoices(buy.riven).data?.sell ?? null;
  const middle = band?.middle == null ? null : Math.round(band.middle);
  const number = (value: number | null | undefined) =>
    value == null ? "—" : formatCount(locale, Math.round(value));
  const gap = middle === null ? null : middle - buy.price;
  return (
    <article className="flex flex-col gap-3 rounded-panel border border-line bg-surface p-3">
      <RivenHeading
        riven={buy.riven}
        rank={buy.rank}
        weapon={weapon}
        name={name ?? buy.name}
        appraisal={appraisal}
      />
      {appraisal && (
        <div className="flex flex-col gap-1">
          {appraisal.stats.map((stat) => (
            <StatRow key={`${stat.tag}${stat.isCurse}`} stat={stat} />
          ))}
        </div>
      )}
      {selling ? (
        <PriceRow
          label={t("pick-sold-for")}
          start={middle ?? buy.price}
          onDone={(price) => {
            setSelling(false);
            if (price !== null) sell.mutate({ auction: buy.auction, price });
          }}
        />
      ) : (
        <div className="flex items-center gap-3">
          <Money
            currency="platinum"
            amount={buy.price}
            className="font-semibold text-ink text-title"
          />
          <span className="ml-auto flex">
            <Action hint={t("pick-sold")} onClick={() => setSelling(true)}>
              <HandCoins />
            </Action>
          </span>
        </div>
      )}
      <div className="grid grid-cols-3 gap-2 border-hairline border-t pt-2.5">
        <Mark label={t("auction-market")}>
          {number(middle)}
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
        <Mark label={t("pick-bought-on")}>{formatDay(locale, buy.bought)}</Mark>
      </div>
    </article>
  );
}
