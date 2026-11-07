import type { Offered } from "@median/ipc";
import { Star } from "lucide-react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatTenths } from "@/shared/lib/format";
import { ItemPicture } from "@/shared/ui/item-picture";

/** Placeholder for a value the market cache does not have. */
const NONE = "—";

function amount(value: number | null): string {
  return value === null ? NONE : String(value);
}

/**
 * One reward card of the reward screen: name, platinum and ducat value, trades per day, the
 * account's progress on its set, and whether the set's item is built or mastered. A platinum value
 * prefixed `~` is the closed-trade price, shown when no in-game seller price is known.
 */
export function RewardCard({ card, fresh }: { card: Offered; fresh: boolean }) {
  const t = useT();
  const locale = useLocale();
  return (
    <div className="flex flex-col gap-2.5 rounded-lg bg-white/5 p-3">
      <div className="relative min-h-9 pr-6 font-medium text-sm text-soft leading-snug">
        {card.name}
        {card.favourite && (
          <Star className="absolute top-0.5 right-2.5 size-3 fill-plat text-plat" />
        )}
        {card.own && <span className="absolute top-1 right-0 size-1.5 rounded-full bg-live" />}
      </div>
      <div className="flex items-baseline gap-3 tabular-nums leading-none">
        <span className={cn("font-bold text-plat text-xl", !fresh && "animate-pulse")}>
          {card.ingame === null && card.platinum !== null && "~"}
          {amount(card.ingame ?? card.platinum)}
        </span>
        <span className="font-semibold text-base text-duc">{amount(card.ducats)}</span>
        {card.perDayTenths !== null && (
          <span className="ml-auto text-muted text-xs">
            {t("overlay-per-day", { count: formatTenths(locale, card.perDayTenths) })}
          </span>
        )}
      </div>
      {card.kit.length > 0 && (
        <>
          <div className="flex gap-1.5">
            {card.kit.map((piece, at) => (
              <span
                // biome-ignore lint/suspicious/noArrayIndexKey: set parts keep catalog order
                key={at}
                className={cn(
                  "relative aspect-square max-w-12 flex-1 rounded-full bg-black/55 ring-1 ring-white/10 ring-inset",
                  piece.this && "ring-2 ring-duc",
                )}
              >
                <ItemPicture
                  image={piece.image}
                  className={cn("size-full p-1", piece.count === 0 && "opacity-35 grayscale")}
                />
                <b
                  className={cn(
                    "absolute -right-0.5 -bottom-0.5 flex h-4 min-w-4 items-center justify-center rounded-full px-0.5 font-bold text-xs tabular-nums leading-none ring-2 ring-black/80",
                    piece.count >= piece.need ? "bg-duc text-canvas" : "bg-sunken text-dim",
                  )}
                >
                  {piece.need > 1 ? `${piece.count}/${piece.need}` : piece.count}
                </b>
              </span>
            ))}
          </div>
          <div className="flex items-baseline justify-between gap-2 text-muted text-xs">
            <span>
              {t("overlay-whole")}{" "}
              <b
                className={cn(
                  "font-semibold text-duc text-sm tabular-nums",
                  !fresh && "animate-pulse",
                )}
              >
                {amount(card.whole)}
              </b>
            </span>
            {card.reached && (
              <span className={card.reached === "mastered" ? "text-live" : "text-soft"}>
                {t(card.reached === "mastered" ? "overlay-mastered" : "overlay-built")}
              </span>
            )}
          </div>
        </>
      )}
    </div>
  );
}
