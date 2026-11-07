import type { Way } from "@median/ipc";

/** A drop source: a place or an enemy, with its location. */
export type DropWay = Extract<Way, { kind: "drop" }>;

/** Planets listed for an enemy; the rest are shown as `+N`. */
const HAUNTS_SHOWN = 2;

/** Enemy level range of the drop's place, as `30–40` or `35`; null when unknown. */
export function levelsOf(way: DropWay): string | null {
  if (way.levelFrom === null || way.levelTo === null) return null;
  return way.levelFrom === way.levelTo ? `${way.levelFrom}` : `${way.levelFrom}–${way.levelTo}`;
}

/** Location text of a drop, widest first: `planet · place · mission`, leaving out a part that
 * another part contains; for an enemy, its name with the planets it appears on most. */
export function placeText(way: DropWay): string {
  if (way.enemy) {
    const shown = way.haunts.slice(0, HAUNTS_SHOWN);
    const more = way.haunts.length - shown.length;
    const planets = shown.length > 0 ? `${shown.join(", ")}${more > 0 ? ` +${more}` : ""}` : "";
    return [way.place, planets].filter(Boolean).join(" · ");
  }
  const words: string[] = [];
  for (const word of [way.planet, way.place, way.mission]) {
    if (!word || words.some((held) => held.includes(word))) continue;
    const within = words.findIndex((held) => word.includes(held));
    if (within === -1) words.push(word);
    else words[within] = word;
  }
  return words.join(" · ");
}
