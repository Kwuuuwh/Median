import type { Translate } from "@/shared/i18n";
import { DAY, HOUR, MINUTE } from "./time";

const numberFormats = new Map<string, Intl.NumberFormat>();

/** Number format of one kind for the locale, created on first use and cached. */
function numberFormat(
  kind: string,
  locale: string,
  options: Intl.NumberFormatOptions,
): Intl.NumberFormat {
  const key = `${kind}:${locale}`;
  let format = numberFormats.get(key);
  if (!format) {
    format = new Intl.NumberFormat(locale, options);
    numberFormats.set(key, format);
  }
  return format;
}

/** A whole number with the locale's digit grouping. */
export function formatCount(locale: string, value: number): string {
  return numberFormat("count", locale, { maximumFractionDigits: 0 }).format(value);
}

/** A value stored in tenths, formatted with one decimal for the locale: `25` → `2.5` in en. */
export function formatTenths(locale: string, tenths: number): string {
  return numberFormat("tenths", locale, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(tenths / 10);
}

/** A count in compact notation: 2, 950, 300 тыс., 1,2 млн. */
export function formatCompact(locale: string, count: number): string {
  return numberFormat("compact", locale, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(count);
}

const dayFormats = new Map<string, Intl.DateTimeFormat>();

/** Short calendar day of a Unix time in seconds: `5 нояб.`, `Nov 5`. */
export function formatDay(locale: string, at: number): string {
  let format = dayFormats.get(locale);
  if (!format) {
    format = new Intl.DateTimeFormat(locale, { day: "numeric", month: "short" });
    dayFormats.set(locale, format);
  }
  return format.format(at * 1000);
}

const dateFormats = new Map<string, Intl.DateTimeFormat>();

/** Calendar date with the year of a Unix time in seconds: `04.11.2026`, `11/4/26`. */
export function formatDate(locale: string, at: number): string {
  let format = dateFormats.get(locale);
  if (!format) {
    format = new Intl.DateTimeFormat(locale, { dateStyle: "short" });
    dateFormats.set(locale, format);
  }
  return format.format(at * 1000);
}

/** An age in seconds as its largest whole unit: minutes, hours or days; `fresh` under a minute. */
export function ageUnit(secs: number): {
  unit: "fresh" | "minutes" | "hours" | "days";
  count: number;
} {
  if (secs < MINUTE) return { unit: "fresh", count: 0 };
  if (secs < HOUR) return { unit: "minutes", count: Math.floor(secs / MINUTE) };
  if (secs < DAY) return { unit: "hours", count: Math.floor(secs / HOUR) };
  return { unit: "days", count: Math.floor(secs / DAY) };
}

/** A span of seconds as a clock: `m:ss` under an hour, `h:mm:ss` above; negative is zero. */
export function formatClock(secs: number): string {
  const whole = Math.max(0, Math.floor(secs));
  const hours = Math.floor(whole / HOUR);
  const minutes = Math.floor((whole % HOUR) / MINUTE);
  const seconds = whole % MINUTE;
  const pad = (value: number) => String(value).padStart(2, "0");
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${minutes}:${pad(seconds)}`;
}

/** A span of seconds in its two largest units: `2 д 5 ч`, `3 ч 10 мин`, `40 мин`; negative is 0. */
export function formatSpan(t: Translate, secs: number): string {
  const left = Math.max(0, secs);
  const days = Math.floor(left / DAY);
  const hours = Math.floor((left % DAY) / HOUR);
  const minutes = Math.floor((left % HOUR) / MINUTE);
  if (days > 0) return t("span-days-hours", { days, hours });
  if (hours > 0) return t("span-hours-minutes", { hours, minutes });
  return t("span-minutes", { minutes });
}
