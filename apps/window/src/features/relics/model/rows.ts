import type { Relic, Step } from "@median/ipc";
import { compareMeasures, type Sorting } from "@/shared/lib/sort";

/** Sort keys of the relic list. */
export type RelicOrder = "platinum" | "upgrade" | "ducats" | "count";

/** Refinements of a relic the account holds at least one of, intact first. */
export function keptSteps(relic: Relic): Step[] {
  return relic.steps.filter((step) => step.count > 0);
}

/** Copies of a relic the account holds, all refinements combined. */
export function heldCount(relic: Relic): number {
  return relic.steps.reduce((sum, step) => sum + step.count, 0);
}

/**
 * Average run platinum gained per echo spent refining this step to radiant, in tenths; null for
 * radiant itself or when the relic has no radiant step.
 */
export function gainPerEcho(relic: Relic, step: Step): number | null {
  const radiant = relic.steps.find((each) => each.refinement === "radiant");
  if (!radiant || radiant.echoes <= step.echoes) return null;
  return (radiant.platinum - step.platinum) / (radiant.echoes - step.echoes);
}

/** Sort value of a held refinement for a sort key. */
function measure(relic: Relic, step: Step, order: Exclude<RelicOrder, "count">): number {
  switch (order) {
    case "platinum":
      return step.platinum;
    case "ducats":
      return step.ducats;
    case "upgrade":
      return gainPerEcho(relic, step) ?? 0;
  }
}

/** Highest sort value among the relic's held refinements; 0 when none is held. */
function best(relic: Relic, order: Exclude<RelicOrder, "count">): number {
  return Math.max(0, ...keptSteps(relic).map((step) => measure(relic, step, order)));
}

/** Eras present in `relics`, in the given `order` (the game's unlock order). */
export function erasOf(relics: readonly Relic[], order: readonly string[]): string[] {
  const present = new Set(relics.map((relic) => relic.era));
  return order.filter((era) => present.has(era));
}

/**
 * Held relics of the era (any era for null) whose name or a reward name matches the search,
 * sorted by the chosen key and direction; ties by name.
 */
export function relicRows(
  relics: readonly Relic[],
  era: string | null,
  sorting: Sorting<RelicOrder>,
  matches: (name: string) => boolean,
): Relic[] {
  const { by, descending } = sorting;
  const key = (relic: Relic) => (by === "count" ? heldCount(relic) : best(relic, by));
  return relics
    .filter((relic) => heldCount(relic) > 0)
    .filter((relic) => era === null || relic.era === era)
    .filter((relic) => matches(relic.name) || relic.holds.some((slot) => matches(slot.name)))
    .sort((a, b) => compareMeasures(key(a), key(b), descending) || a.name.localeCompare(b.name));
}
