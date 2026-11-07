import type { Relic, Step } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { erasOf, gainPerEcho, heldCount, keptSteps, relicRows } from "./rows";

const ECHOES: Record<string, number> = { intact: 0, exceptional: 25, flawless: 50, radiant: 100 };

function step(refinement: string, count: number, platinum: number, ducats: number): Step {
  return {
    item: `/${refinement}`,
    refinement,
    count,
    platinum,
    ducats,
    echoes: ECHOES[refinement] ?? 0,
  };
}

function relic(name: string, era: string, steps: Step[]): Relic {
  return { base: name, name, image: null, era, vaulted: false, steps, holds: [] };
}

const relics = [
  relic("Axi A1", "axi", [step("intact", 1, 50, 300), step("radiant", 2, 90, 400)]),
  relic("Lith G1", "lith", [step("intact", 9, 70, 100), step("radiant", 0, 200, 200)]),
  relic("Neo N1", "neo", [step("intact", 0, 300, 900)]),
];
const all = () => true;

describe("the relic list", () => {
  it("holds only relics the account has, refinements it has", () => {
    expect(relicRows(relics, null, { by: "platinum", descending: true }, all)).toHaveLength(2);
    expect(keptSteps(relics[0])).toHaveLength(2);
    expect(heldCount(relics[0])).toBe(3);
  });

  it("orders by the best held payback in platinum, in ducats, or by copies", () => {
    expect(relicRows(relics, null, { by: "platinum", descending: true }, all)[0].name).toBe(
      "Axi A1",
    );
    expect(relicRows(relics, null, { by: "ducats", descending: true }, all)[0].name).toBe("Axi A1");
    expect(relicRows(relics, null, { by: "count", descending: true }, all)[0].name).toBe("Lith G1");
  });

  it("orders by what refining gains per echo", () => {
    expect(relicRows(relics, null, { by: "upgrade", descending: true }, all)[0].name).toBe(
      "Lith G1",
    );
  });

  it("gains nothing by refining a radiant relic", () => {
    const [intact, radiant] = relics[1].steps;
    expect(gainPerEcho(relics[1], intact)).toBeCloseTo(1.3);
    expect(gainPerEcho(relics[1], radiant)).toBeNull();
  });

  it("narrows by era", () => {
    expect(
      relicRows(relics, "lith", { by: "platinum", descending: true }, all).map((each) => each.name),
    ).toEqual(["Lith G1"]);
    expect(erasOf(relics, ["lith", "meso", "neo", "axi"])).toEqual(["lith", "neo", "axi"]);
  });
});
