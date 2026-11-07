import type { Worldstate } from "@median/ipc";

/** The recorded world state shifted in time as if fetched at `now`; all time offsets are kept. */
export function worldAsOf(world: Worldstate, now: number): Worldstate {
  const shift = (at: number) => at + now - world.at;
  return {
    at: now,
    cycles: world.cycles.map((cycle) => ({ ...cycle, ends: shift(cycle.ends) })),
    fissures: world.fissures.map((fissure) => ({ ...fissure, expires: shift(fissure.expires) })),
    deadlines: world.deadlines.map((deadline) => ({ ...deadline, at: shift(deadline.at) })),
    visits: world.visits.map((visit) => ({
      ...visit,
      arrives: shift(visit.arrives),
      departs: shift(visit.departs),
    })),
  };
}
