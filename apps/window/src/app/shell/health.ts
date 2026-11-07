import type { MessageId } from "@median/i18n";
import { type AppState, COLLECTOR_GRACE_S } from "@median/ipc";

/**
 * Problem source: collector and Overwolf game-event feeds (GEP), game log (DBWIN), reward
 * recognition (OCR), warframe.market, or a background task.
 */
export type Source = "gep" | "dbwin" | "ocr" | "wfm" | "task";

/** One current problem with a data source. */
export type Trouble = {
  source: Source;
  /** Message id describing the problem. */
  said: MessageId;
  /** Whether the dependent feature has stopped, rather than working worse. */
  severe: boolean;
  /** Task name, for a task problem. */
  name?: string;
  /** Error text reported by the source, when there is one. */
  reason?: string;
  /** Unix time in seconds the problem started, when known. */
  since?: number;
  /** Whether the problem is fixed by loading the collector into Overwolf. */
  loadsCollector?: boolean;
};

/**
 * Current problems with the app's data sources and background tasks at Unix time `now` in
 * seconds; empty when there are none. A closed game without a collector is not a problem; a
 * running game is, once it has run `COLLECTOR_GRACE_S` seconds without one.
 */
export function troublesOf(state: AppState, now: number): Trouble[] {
  const troubles: Trouble[] = [];
  const { collector, game } = state;
  if (collector.state === "outdated") {
    troubles.push({ source: "gep", said: "trouble-gep-outdated", severe: true });
  } else if (collector.state === "live" && collector.feeds.state === "refused") {
    troubles.push({
      source: "gep",
      said: "trouble-gep-refused",
      severe: true,
      reason: collector.feeds.reason || undefined,
    });
  } else if (
    collector.state === "offline" &&
    game.runningSince !== null &&
    now - game.runningSince >= COLLECTOR_GRACE_S
  ) {
    troubles.push({
      source: "gep",
      said: "trouble-collector-missing",
      severe: true,
      loadsCollector: true,
      since: game.runningSince,
    });
  }
  if (!game.listening) {
    troubles.push({ source: "dbwin", said: "trouble-dbwin-closed", severe: true });
  } else if (game.shared) {
    troubles.push({ source: "dbwin", said: "trouble-dbwin-shared", severe: false });
  }
  if (game.recognition === "failed") {
    troubles.push({ source: "ocr", said: "trouble-ocr-failed", severe: true });
  }
  const silent = state.market.failingSince;
  if (typeof silent === "number") {
    troubles.push({ source: "wfm", said: "trouble-wfm-silent", severe: true, since: silent });
  }
  for (const task of state.tasks) {
    if (task.state.kind === "running") continue;
    troubles.push({
      source: "task",
      said: task.state.kind === "stopped" ? "trouble-task-stopped" : "trouble-task-restarting",
      severe: task.state.kind === "stopped",
      name: task.name,
      reason: task.state.reason,
    });
  }
  return troubles;
}
