import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { formatCount } from "@/shared/lib/format";
import type { Money as Coin } from "@/shared/ui/currency-icon";
import { Money } from "@/shared/ui/money";

/** English catalog currency names that have a currency icon in the window. */
const COIN: Record<string, Coin> = {
  Platinum: "platinum",
  Ducats: "ducats",
  "Orokin Ducats": "ducats",
  Aya: "aya",
  "Regal Aya": "regalAya",
};

/** A vendor price with its currency icon when the window has one, else with the currency name. */
export function VendorCost({
  cost,
  currency,
  coin: named = null,
}: {
  cost: number;
  currency: string;
  /** The currency's name in the interface language, when the catalog has it as an item. */
  coin?: string | null;
}) {
  const t = useT();
  const locale = useLocale();
  const coin = COIN[currency];
  if (coin) return <Money currency={coin} amount={cost} />;
  return (
    <span className="tabular-nums">
      {formatCount(locale, cost)}{" "}
      {currency === "Standing" ? t("mastery-standing") : (named ?? currency)}
    </span>
  );
}
