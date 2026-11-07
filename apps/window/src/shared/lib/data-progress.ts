import { formatBytes } from "@median/i18n";
import type { Catalog } from "@median/ipc";
import type { Translate } from "@/shared/i18n";

/** What the game data is doing: a title, a detail line and whether a retry helps. */
export function dataProgress(
  t: Translate,
  locale: string,
  catalog: Catalog,
): { title: string; detail?: string; retry?: boolean } {
  const activity = catalog.activity;
  switch (activity.state) {
    case "checking":
      return { title: t("gate-checking") };
    case "fetching":
      return {
        title: t(activity.part === "database" ? "gate-fetching" : "gate-pictures"),
        detail: `${formatBytes(locale, activity.done)} / ${formatBytes(locale, activity.total)}`,
      };
    case "opening":
      return { title: t("gate-installing") };
    case "unreachable":
      return { title: t("gate-offline-title"), detail: t("gate-offline-hint"), retry: true };
    case "unsupported":
      return { title: t("gate-outdated-title"), detail: t("gate-outdated-hint") };
    case "stale":
      return { title: t("gate-stale-title"), detail: t("gate-stale-hint") };
    case "idle":
      return { title: t("gate-preparing") };
  }
}
