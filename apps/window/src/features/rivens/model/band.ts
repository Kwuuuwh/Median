import type { RivenBand } from "@median/ipc";
import type { Translate } from "@/shared/i18n";

/** Translated text of a price band: its basis, low and high price, and the auction count. */
export function bandText(
  t: Translate,
  band: RivenBand,
  number: (value: number | null | undefined) => string,
): string {
  return t("riven-band", {
    basis: band.basis,
    low: number(band.low),
    high: number(band.high),
    lots: band.lots,
  });
}
