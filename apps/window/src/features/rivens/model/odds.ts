import type { RerollOdds } from "@median/ipc";
import { formatCount } from "@/shared/lib/format";

/** Smallest chance shown as a number (0.1%); a smaller positive chance shows as `<0.1%`. */
const LEAST_CHANCE = 0.001;

/** Expected kuva above which a roll counts as out of reach and shows as `>` this amount. */
const KUVA_REACH = 200_000;

/** A chance as a percent: whole from 1%, one decimal below 1%, `<0.1%` below 0.1%. */
export function chanceText(locale: string, chance: number): string {
  if (chance > 0 && chance < LEAST_CHANCE) {
    return `<${percent(locale, LEAST_CHANCE, 1)}`;
  }
  return percent(locale, chance, chance > 0 && chance < 0.01 ? 1 : 0);
}

/** Expected kuva cost, capped at `>KUVA_REACH`; a dash when no better roll was observed. */
export function kuvaText(locale: string, kuva: number | null): string {
  if (kuva === null) return "—";
  return kuva > KUVA_REACH ? `>${formatCount(locale, KUVA_REACH)}` : formatCount(locale, kuva);
}

function percent(locale: string, share: number, digits: number): string {
  return new Intl.NumberFormat(locale, {
    style: "percent",
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(share);
}

/** The lock option with the lowest expected kuva to a better roll; undefined when none has one. */
export function bestLock(odds: readonly RerollOdds[]): RerollOdds | undefined {
  return odds
    .filter((each) => each.expectedKuva !== null)
    .reduce<RerollOdds | undefined>(
      (best, each) =>
        !best || (each.expectedKuva ?? Infinity) < (best.expectedKuva ?? Infinity) ? each : best,
      undefined,
    );
}
