import type { Currency } from "@median/ipc";
import type { ReactNode } from "react";
import { useT } from "@/shared/i18n";
import { Money } from "@/shared/ui/money";
import { SearchField } from "@/shared/ui/search-field";
import { Tooltip } from "@/shared/ui/tooltip";

/** Header of a vendor list: search, extra controls in `children`, and the account's currency. */
export function ShelfHead({
  search,
  onSearch,
  wallet,
  children,
}: {
  search: string;
  onSearch: (search: string) => void;
  wallet: { currency: Currency; amount: number } | null;
  children?: ReactNode;
}) {
  const t = useT();
  return (
    <div className="flex flex-wrap items-center gap-4">
      <SearchField value={search} onChange={onSearch} placeholder={t("shelf-search")} />
      {children}
      {wallet && (
        <Tooltip hint={t("vendor-currency", { currency: wallet.currency })}>
          <span className="ml-auto inline-flex h-8 items-center rounded-control border border-line px-2.5">
            <Money
              currency={wallet.currency}
              amount={wallet.amount}
              className="font-medium text-body text-ink"
            />
          </span>
        </Tooltip>
      )}
    </div>
  );
}
