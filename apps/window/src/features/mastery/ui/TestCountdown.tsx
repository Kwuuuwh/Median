import { useT } from "@/shared/i18n";
import { formatSpan } from "@/shared/lib/format";
import { useNow } from "@/shared/lib/use-now";

/** Time left until the mastery rank test is available again, ticking every second. */
export function TestCountdown({ until }: { until: number }) {
  const t = useT();
  return <>{t("mastery-test-shut", { left: formatSpan(t, until - useNow()) })}</>;
}
