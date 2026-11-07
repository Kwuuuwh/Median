import type { RerollOdds, RivenBand } from "@median/ipc";
import type { ReactNode } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { Money } from "@/shared/ui/money";
import { Skeleton } from "@/shared/ui/skeleton";
import { bandText } from "../model/band";
import { bestLock, kuvaText } from "../model/odds";

/** One option for a riven: label, platinum value (undefined while loading), and a basis note. */
function Decision({
  label,
  amount,
  note,
  isBest,
}: {
  label: string;
  amount: number | null | undefined;
  note: ReactNode;
  isBest: boolean;
}) {
  return (
    <div
      className={cn(
        "grid gap-1 rounded-control border px-3 py-2.5",
        isBest ? "border-accent/60 bg-accent/6" : "border-line",
      )}
    >
      <span className="text-caption text-muted">{label}</span>
      {amount === undefined ? (
        <Skeleton className="h-6 w-20" />
      ) : (
        <Money currency="platinum" amount={amount} className="font-semibold text-plat text-title" />
      )}
      <span className="h-4 truncate text-caption text-dim">{note}</span>
    </div>
  );
}

/**
 * Platinum value of a riven if sold, rerolled with the best lock, or transmuted; the highest is
 * marked once all three have loaded. Each input is undefined while loading.
 */
export function Decisions({
  sell,
  rolls,
  transmute,
}: {
  sell: RivenBand | null | undefined;
  rolls: readonly RerollOdds[] | null | undefined;
  transmute: number | null | undefined;
}) {
  const t = useT();
  const locale = useLocale();
  const best = rolls ? bestLock(rolls) : undefined;
  const amounts = {
    sell: sell === undefined ? undefined : (sell?.middle ?? null),
    roll: rolls === undefined ? undefined : (best?.betterMean ?? null),
    transmute,
  };
  const all = Object.values(amounts);
  const top = Math.max(0, ...all.map((amount) => amount ?? 0));
  const isBest = (amount: number | null | undefined) =>
    !all.includes(undefined) && amount != null && amount === top;
  const number = (value: number | null | undefined) => formatCount(locale, value ?? 0);
  return (
    <div className="grid grid-cols-3 gap-2">
      <Decision
        label={t("riven-sell")}
        amount={amounts.sell}
        note={sell && bandText(t, sell, number)}
        isBest={isBest(amounts.sell)}
      />
      <Decision
        label={t("riven-reroll")}
        amount={amounts.roll}
        note={best && t("riven-kuva", { amount: kuvaText(locale, best.expectedKuva ?? null) })}
        isBest={isBest(amounts.roll)}
      />
      <Decision
        label={t("riven-transmute")}
        amount={amounts.transmute}
        note=""
        isBest={isBest(amounts.transmute)}
      />
    </div>
  );
}
