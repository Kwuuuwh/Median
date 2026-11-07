import type { Agent } from "@median/ipc";
import type { Blocker } from "./agent";

/** Time window the pace is measured over, in seconds. */
const PACE_SECS = 300;

/** Seconds without a newly evaluated item after which the agent counts as waiting. */
const QUIET_SECS = 120;

/** Agent activity: off, blocked, evaluating items, or waiting for market activity. */
export type Pulse = "off" | "blocked" | "looking" | "waiting";

/** One sample of the agent's running count of evaluated items, taken at Unix second `at`. */
export type Count = { at: number; seen: number };

/** Items evaluated per minute over the last 5 minutes of samples; null with fewer than two. */
export function paceOf(counts: readonly Count[], now: number): number | null {
  const recent = counts.filter((count) => now - count.at <= PACE_SECS);
  if (recent.length < 2) return null;
  const first = recent[0];
  const last = recent[recent.length - 1];
  const secs = last.at - first.at;
  return secs > 0 ? Math.round(((last.seen - first.seen) * 60) / secs) : null;
}

/** Current agent activity; `looking` when it evaluated an item in the last 2 minutes. */
export function pulseOf(agent: Agent, blockers: readonly Blocker[], now: number): Pulse {
  if (!agent.running) return "off";
  if (blockers.length > 0) return "blocked";
  const last = agent.flow.reduce((latest, seen) => Math.max(latest, seen.at), 0);
  return now - last <= QUIET_SECS ? "looking" : "waiting";
}
