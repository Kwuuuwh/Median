import type { Locale } from "./locale";

/// Units for byte counts, smallest first; each is 1000 times the previous one.
const UNITS = ["byte", "kilobyte", "megabyte", "gigabyte"] as const;

const formatters = new Map<string, Intl.NumberFormat>();

function formatter(locale: Locale, step: number): Intl.NumberFormat {
  const key = `${locale}:${step}`;
  let cached = formatters.get(key);
  if (!cached) {
    cached = new Intl.NumberFormat(locale, {
      style: "unit",
      unit: UNITS[step],
      // In English the short form of the base unit spells out "byte", so it uses the narrow
      // form; the larger units use the short form, which keeps the space before the unit.
      unitDisplay: step === 0 ? "narrow" : "short",
      maximumFractionDigits: step === 0 ? 0 : 1,
    });
    formatters.set(key, cached);
  }
  return cached;
}

/// A byte count in the largest unit that keeps the value under 1000 (up to gigabytes), localised.
export function formatBytes(locale: Locale, bytes: number): string {
  let value = bytes;
  let step = 0;
  while (value >= 1000 && step < UNITS.length - 1) {
    value /= 1000;
    step += 1;
  }
  return formatter(locale, step).format(value);
}
