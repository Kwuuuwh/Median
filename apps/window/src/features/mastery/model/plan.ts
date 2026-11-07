import type { Masterable, Planned, Requirement } from "@median/ipc";

/** Plan steps, in page order: level, build in the foundry, gather, open relics, obtain. */
export const NEEDS = ["level", "forge", "resources", "relics", "obtain"] as const;

export type Need = (typeof NEEDS)[number];

/** Item kinds gathered in stacks rather than obtained one at a time. */
const RESOURCE_KINDS: ReadonlySet<string> = new Set([
  "resource",
  "misc-resource",
  "railjack-resource",
  "research-sample",
  "gem",
  "plant",
  "fish",
  "fish-part",
  "fish-bait",
  "eidolon-shard",
  "currency",
  "dog-tag",
]);

/** Copies covered neither by the inventory nor by builds in the foundry; never negative. */
export function shortOf(line: Requirement): number {
  return Math.max(0, line.count - line.have - line.forging);
}

/** Whether a line still needs work: it is built in the foundry, or copies are missing. */
function waits(line: Requirement): boolean {
  return line.built || shortOf(line) > 0;
}

/** The step of a plan line: `forge` when the lines under it build it, `relics` when a relic drops
 * it or dropped it before vaulting, `resources` for a gathered kind, `obtain` otherwise. */
function needOf(line: Requirement): Exclude<Need, "level"> {
  if (line.built) return "forge";
  if (line.relicNames.length > 0 || line.relics > 0 || line.ownedRelics > 0 || line.vaulted)
    return "relics";
  if (RESOURCE_KINDS.has(line.kind)) return "resources";
  return "obtain";
}

/** Acquisition groups, in page order. */
const GETS = ["credits", "trader", "lab", "drop", "unknown"] as const;

export type Get = (typeof GETS)[number];

/** Acquisition group of a line, from its easiest way. */
function getOf(line: Requirement): Get {
  switch (line.way?.kind) {
    case "credits":
      return "credits";
    case "vendor":
    case "visit":
      return "trader";
    case "lab":
      return "lab";
    case "drop":
      return "drop";
    default:
      return "unknown";
  }
}

/** Lines grouped by acquisition, each group in input order; empty groups are omitted. */
export function byGet<T extends Requirement>(lines: readonly T[]): { get: Get; lines: T[] }[] {
  return GETS.map((get) => ({ get, lines: lines.filter((line) => getOf(line) === get) })).filter(
    (group) => group.lines.length > 0,
  );
}

/** Whether a line is obtained in the area; always true without an area and for foundry lines. */
function within(line: Requirement, area: string | null): boolean {
  return area === null || line.built || line.areas.includes(area);
}

/** Lines of a step that still need work: startable builds first for `forge`, most copies missing
 * first for `resources`, lines with held relics first for `relics`; then by name. */
export function needLines(
  lines: readonly Requirement[],
  need: Exclude<Need, "level">,
  area: string | null,
  startable: ReadonlySet<string>,
): Requirement[] {
  const order = (a: Requirement, b: Requirement) => {
    switch (need) {
      case "forge":
        return Number(startable.has(b.item)) - Number(startable.has(a.item));
      case "resources":
        return shortOf(b) - shortOf(a);
      case "relics":
        return Number(b.ownedRelics > 0) - Number(a.ownedRelics > 0);
      default:
        return 0;
    }
  };
  return lines
    .filter((line) => waits(line) && needOf(line) === need && within(line, area))
    .sort((a, b) => order(a, b) || a.name.localeCompare(b.name));
}

/** A held planned item with its current rank and rank cap. */
export type Levelling = { planned: Planned; rank: number; cap: number };

/** Held planned items below their rank cap, closest to the cap first, then by name. */
export function levelling(
  planned: readonly Planned[],
  masterable: readonly Masterable[],
): Levelling[] {
  const known = new Map(masterable.map((item) => [item.item, item]));
  return planned
    .filter((each) => each.owned)
    .map((each) => ({
      planned: each,
      rank: known.get(each.item)?.rank ?? 0,
      cap: known.get(each.item)?.cap ?? 0,
    }))
    .filter((each) => each.cap > 0 && each.rank < each.cap)
    .sort(
      (a, b) => b.rank / b.cap - a.rank / a.cap || a.planned.name.localeCompare(b.planned.name),
    );
}

/**
 * Lines that still need work per step, with `levelled` as the `level` count; with an area, only
 * lines obtained there.
 */
export function needCounts(
  lines: readonly Requirement[],
  area: string | null,
  levelled: number,
): Map<Need, number> {
  const counts = new Map<Need, number>([["level", levelled]]);
  for (const line of lines) {
    if (!waits(line) || !within(line, area)) continue;
    const need = needOf(line);
    counts.set(need, (counts.get(need) ?? 0) + 1);
  }
  return counts;
}
