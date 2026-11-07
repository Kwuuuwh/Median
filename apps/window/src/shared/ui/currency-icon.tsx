import type { Currency } from "@median/ipc";
import { cn } from "@/shared/lib/cn";

/** Every currency a price can be in. */
export type Money = Currency | "platinum";

/** Picture of each currency, bundled with the window. */
const PICTURE: Record<Money, string> = {
  platinum: "/currency/platinum.webp",
  credits: "/currency/credits.webp",
  ducats: "/currency/ducats.webp",
  aya: "/currency/aya.webp",
  regalAya: "/currency/regal_aya.webp",
};

/** Decorative 16 px icon of a currency. */
export function CurrencyIcon({ currency, className }: { currency: Money; className?: string }) {
  return (
    <img
      src={PICTURE[currency]}
      alt=""
      draggable={false}
      className={cn("size-4 shrink-0 object-contain", className)}
    />
  );
}
