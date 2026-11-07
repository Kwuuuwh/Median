import type { Era, Fissure, Watch } from "@median/ipc";

/** Which fissures are listed: all, normal only, or Steel Path only. */
export type FissureScope = "all" | "plain" | "hard";

/** Eras in display order: the void eras, then Omnia, then Requiem. */
export const ERAS: readonly Era[] = ["lith", "meso", "neo", "axi", "omnia", "requiem"];

/** Fissure list filter; an empty era or mission set allows every era or mission. */
export type FissureFilter = {
  eras: ReadonlySet<Era>;
  scope: FissureScope;
  missions: ReadonlySet<string>;
  railjack: boolean;
};

export const NO_FILTER: FissureFilter = {
  eras: new Set(),
  scope: "all",
  missions: new Set(),
  railjack: false,
};

/** Whether the fissure is open and passes the filter; Railjack storms only with `railjack`. */
export function isListed(fissure: Fissure, filter: FissureFilter, now: number): boolean {
  if (fissure.expires <= now) return false;
  if (fissure.storm && !filter.railjack) return false;
  if (filter.eras.size > 0 && !filter.eras.has(fissure.era)) return false;
  if (filter.scope === "plain" && fissure.hard) return false;
  if (filter.scope === "hard" && !fissure.hard) return false;
  return (
    filter.missions.size === 0 || (fissure.mission !== null && filter.missions.has(fissure.mission))
  );
}

/** Fissures that pass `isListed`, the one closing soonest first. */
export function listFissures(
  fissures: readonly Fissure[],
  filter: FissureFilter,
  now: number,
): Fissure[] {
  return fissures
    .filter((fissure) => isListed(fissure, filter, now))
    .sort((a, b) => a.expires - b.expires);
}

/**
 * Watches equivalent to the filter: one per picked era and mission combination, with null (any)
 * where nothing is picked.
 */
export function watchesFor(filter: FissureFilter): Watch[] {
  const of = filter.scope === "all" ? null : filter.scope;
  const eras: (Era | null)[] =
    filter.eras.size > 0 ? ERAS.filter((era) => filter.eras.has(era)) : [null];
  const missions: (string | null)[] =
    filter.missions.size > 0 ? [...filter.missions].sort() : [null];
  return eras.flatMap((era) =>
    missions.map((mission): Watch => ({ kind: "fissure", era, mission, of })),
  );
}

/** Whether the filter restricts era, scope or mission, so it can become a watch. */
export function narrows(filter: FissureFilter): boolean {
  return filter.eras.size > 0 || filter.scope !== "all" || filter.missions.size > 0;
}
