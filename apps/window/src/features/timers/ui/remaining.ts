import { useT } from "@/shared/i18n";
import { formatClock, formatSpan } from "@/shared/lib/format";
import { DAY } from "@/shared/lib/time";

/** Formatter of time left in seconds: a clock under a day, the two largest units from a day up. */
export function useRemaining(): (secs: number) => string {
  const t = useT();
  return (secs) => (secs < DAY ? formatClock(secs) : formatSpan(t, secs));
}
