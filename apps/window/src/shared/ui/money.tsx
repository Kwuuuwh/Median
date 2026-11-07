import { useLocale } from "@/shared/data/app-state";
import { cn } from "@/shared/lib/cn";
import { formatCount, formatTenths } from "@/shared/lib/format";
import { type Money as Currency, CurrencyIcon } from "./currency-icon";

/** An amount with its currency icon before it; a dash when null. With `tenths`, the amount is
 * in tenths and shown with one decimal. */
export function Money({
  currency,
  amount,
  tenths = false,
  className,
}: {
  currency: Currency;
  amount: number | null;
  tenths?: boolean;
  className?: string;
}) {
  const locale = useLocale();
  return (
    <span
      className={cn("inline-flex items-center gap-1 whitespace-nowrap tabular-nums", className)}
    >
      <CurrencyIcon currency={currency} />
      {amount === null ? (
        <span className="text-dim">—</span>
      ) : tenths ? (
        formatTenths(locale, amount)
      ) : (
        formatCount(locale, amount)
      )}
    </span>
  );
}
