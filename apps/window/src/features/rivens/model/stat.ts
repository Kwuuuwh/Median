import type { StatForm } from "@median/ipc";

/** A riven stat: its value and the in-game label written after the number. */
export type Written = { tag: string; form: StatForm; amount: number | null; label: string };

/** A stat as the game writes it: `+274.4% урона`, `x1.58 урона Корпусу`, `-1.5 к пронзанию`. */
export function statText(stat: Written, locale: string): string {
  const number = (value: number, digits: number) =>
    value.toLocaleString(locale, { minimumFractionDigits: digits, maximumFractionDigits: digits });
  const signed = (value: number, digits: number) =>
    `${value < 0 ? "−" : "+"}${number(Math.abs(value), digits)}`;
  const amount = stat.amount ?? 0;
  switch (stat.form) {
    case "multiplier":
      return `x${number(1 + amount, 2)} ${stat.label}`;
    case "plain":
      return `${signed(amount, 1)} ${stat.label}`;
    default:
      return `${signed(amount * 100, 1)}${stat.label}`;
  }
}

/**
 * Letter grade of a stat roll by its position in the stat's range, from 0 to 1: A in the top
 * third, B in the middle, C in the bottom.
 */
export function rollLetter(share: number): "A" | "B" | "C" {
  if (share >= 2 / 3) return "A";
  if (share >= 1 / 3) return "B";
  return "C";
}
