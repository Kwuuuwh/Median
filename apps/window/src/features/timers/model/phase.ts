import type { Cycle } from "@median/ipc";

/**
 * The cycle phase at `now`, its end time and the next phase, advanced from the phase the backend
 * last reported; null for a cycle without phases.
 */
export function currentPhase(
  cycle: Cycle,
  now: number,
): { phase: string; ends: number; next: string } | null {
  const count = cycle.phases.length;
  if (count === 0) return null;
  let index = cycle.phase;
  let ends = cycle.ends;
  while (ends <= now) {
    index = (index + 1) % count;
    ends += cycle.phases[index].seconds;
  }
  return { phase: cycle.phases[index].key, ends, next: cycle.phases[(index + 1) % count].key };
}
