import { Check, X } from "lucide-react";
import { type KeyboardEvent, useState } from "react";
import { Action } from "@/features/orders/ui/OrderTile";
import { useT } from "@/shared/i18n";
import { Input } from "@/shared/ui/input";
import { wholeNumber } from "@/shared/ui/number-field";

/** Highest riven price the field accepts, in platinum. */
const MOST = 100_000;

/** Inline price field: Enter confirms a valid price, Escape cancels with null. */
export function PriceRow({
  label,
  start,
  onDone,
}: {
  label: string;
  start: number;
  onDone: (price: number | null) => void;
}) {
  const t = useT();
  const [typed, setTyped] = useState(String(start));
  const price = wholeNumber(typed, 1, MOST);
  const keep = () => price !== null && onDone(price);
  const keys = (event: KeyboardEvent) => {
    if (event.key === "Enter") keep();
    if (event.key === "Escape") onDone(null);
  };
  return (
    <div className="flex items-center gap-2">
      <span className="text-muted text-small">{label}</span>
      <Input
        type="number"
        min={1}
        autoFocus
        aria-label={label}
        aria-invalid={price === null}
        value={typed}
        onChange={(event) => setTyped(event.target.value)}
        onKeyDown={keys}
        className="w-20"
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
