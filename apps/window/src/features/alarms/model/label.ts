import type { Node, Watch } from "@median/ipc";
import type { Translate } from "@/shared/i18n";

/** Translated label of what a watch waits for; the watch list and its alarms use the same label. */
export function watchLabel(t: Translate, watch: Watch): string {
  switch (watch.kind) {
    case "phase":
      return `${t("cycle-name", { cycle: watch.cycle })}: ${t("cycle-phase", { phase: watch.phase })}`;
    case "arrival":
      return `${t("trader-name", { vendor: watch.vendor })}: ${t(watch.vendor === "baro" ? "alarm-arrival" : "alarm-rotation")}`;
    case "fissure":
      return [
        watch.mission ? t("mission-type", { mission: watch.mission }) : null,
        watch.era ? t("era-name", { era: watch.era }) : t("watch-any-era"),
        watch.of ? t(`fissure-watch-${watch.of}`) : null,
      ]
        .filter(Boolean)
        .join(" · ");
  }
}

/**
 * Location of an alarm's node as `mission · node, planet`, without the mission when the node name
 * already starts with it; undefined without a node.
 */
export function alarmPlace(node: Node | undefined): string | undefined {
  if (!node) return undefined;
  return node.mission && !node.name.startsWith(node.mission)
    ? `${node.mission} · ${node.name}, ${node.planet}`
    : `${node.name}, ${node.planet}`;
}
