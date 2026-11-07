import { Package } from "lucide-react";
import { useState } from "react";
import { openItem } from "@/features/items/model/open";
import { ItemThumb } from "@/features/items/ui/ItemThumb";
import { useLocale } from "@/shared/data/app-state";
import { useHaul, useRestartHaul } from "@/shared/data/haul";
import { useT } from "@/shared/i18n";
import { formatCount } from "@/shared/lib/format";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogClose, DialogContent, DialogTitle, DialogTrigger } from "@/shared/ui/dialog";
import { Money } from "@/shared/ui/money";
import { type HaulRow, haulRows, haulTotals } from "../model/haul";

/** Placeholder for a price the market cache does not have yet. */
const NONE = "—";

/** One haul item row: count, price per copy, and total platinum and ducat value. */
function HaulItem({ row }: { row: HaulRow }) {
  const t = useT();
  return (
    <li className="grid grid-cols-[minmax(0,1fr)_6rem_5rem_5rem] items-center gap-x-3 border-hairline border-b py-1.5">
      <button
        type="button"
        onClick={() => openItem(row.item)}
        className="flex min-w-0 items-center gap-2.5 text-left"
      >
        <ItemThumb image={row.image} size="sm" />
        <span className="truncate text-body text-ink hover:text-accent">{row.name}</span>
      </button>
      <span className="text-right text-body text-soft tabular-nums">
        ×{row.count}
        {row.count > 1 && row.each !== null && (
          <span className="ml-1.5 text-muted text-small">
            {t("haul-each", { platinum: row.each })}
          </span>
        )}
      </span>
      {row.platinum === null ? (
        <span className="text-right text-dim">{NONE}</span>
      ) : (
        <Money currency="platinum" amount={row.platinum} className="justify-end text-ink" />
      )}
      <span className="flex justify-end">
        {row.ducats > 0 && <Money currency="ducats" amount={row.ducats} className="text-soft" />}
      </span>
    </li>
  );
}

/**
 * Badge with the copies and value gained since the haul started; opens the full list. Renders
 * nothing while the haul has no rows.
 */
export function HaulBadge() {
  const t = useT();
  const locale = useLocale();
  const haul = useHaul().data;
  const restart = useRestartHaul();
  const [open, setOpen] = useState(false);
  const rows = haul ? haulRows(haul.lines) : [];
  if (rows.length === 0) return null;

  const sum = haulTotals(rows);
  const copies = rows.reduce((total, row) => total + row.count, 0);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <button
          type="button"
          aria-label={t("haul-title")}
          className="absolute bottom-4 left-6 inline-flex items-center gap-3 rounded-full border border-line bg-raised/90 px-3.5 py-1.5 shadow-black/40 shadow-lg backdrop-blur-sm hover:border-dim"
        >
          <span className="inline-flex items-center gap-1 text-body text-soft tabular-nums">
            <Package className="size-4" />
            {formatCount(locale, copies)}
          </span>
          <Money
            currency="platinum"
            amount={sum.platinum}
            className="font-medium text-body text-ink"
          />
          {sum.ducats > 0 && (
            <Money currency="ducats" amount={sum.ducats} className="text-body text-soft" />
          )}
        </button>
      </DialogTrigger>
      <DialogContent className="flex max-h-[80vh] w-[36rem] flex-col gap-3">
        <DialogTitle>{t("haul-title")}</DialogTitle>
        <ul className="min-h-0 flex-1 overflow-y-auto">
          {rows.map((row) => (
            <HaulItem key={`${row.item}:${row.variant}`} row={row} />
          ))}
        </ul>
        <div className="flex items-center gap-3">
          <span className="text-body text-soft tabular-nums">×{formatCount(locale, copies)}</span>
          <Money
            currency="platinum"
            amount={sum.platinum}
            className="font-medium text-body text-ink"
          />
          {sum.ducats > 0 && (
            <Money currency="ducats" amount={sum.ducats} className="text-body text-soft" />
          )}
          <span className="flex-1" />
          <Button
            tone="danger"
            onClick={() => {
              restart.mutate();
              setOpen(false);
            }}
          >
            {t("haul-close")}
          </Button>
          <DialogClose asChild>
            <Button tone="primary">{t("haul-continue")}</Button>
          </DialogClose>
        </div>
      </DialogContent>
    </Dialog>
  );
}
