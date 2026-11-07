import type { Agent } from "@median/ipc";

/**
 * Progress of fetching market data for the agent's watched items: `counting` until the watched
 * items are counted, `warming` while some are not fetched yet, `warm` once all are.
 */
export type Warmth =
  | { phase: "counting" }
  | { phase: "warming" | "warm"; warm: number; hot: number; share: number };

/** Fetch progress of the watched items; `share` is the fetched fraction, from 0 to 1. */
export function warmthOf(agent: Pick<Agent, "warm" | "hot">): Warmth {
  if (agent.hot === 0) return { phase: "counting" };
  const warm = Math.min(agent.warm, agent.hot);
  return {
    phase: warm >= agent.hot ? "warm" : "warming",
    warm,
    hot: agent.hot,
    share: warm / agent.hot,
  };
}
