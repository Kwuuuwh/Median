import type { Fissure } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { isListed, listFissures, NO_FILTER, narrows, watchesFor } from "./fissures";

const fissure = (change: Partial<Fissure>): Fissure => ({
  node: "SolNode1",
  era: "lith",
  hard: false,
  storm: false,
  mission: "MT_DEFENSE",
  expires: 100,
  ...change,
});

describe("isListed", () => {
  it("keeps Railjack storms out unless asked for", () => {
    expect(isListed(fissure({ storm: true }), NO_FILTER, 0)).toBe(false);
    expect(isListed(fissure({ storm: true }), { ...NO_FILTER, railjack: true }, 0)).toBe(true);
  });
  it("narrows by era, scope and mission together", () => {
    const filter = {
      ...NO_FILTER,
      eras: new Set(["axi"] as const),
      scope: "hard" as const,
      missions: new Set(["MT_DEFENSE"]),
    };
    expect(isListed(fissure({ era: "axi", hard: true }), filter, 0)).toBe(true);
    expect(isListed(fissure({ era: "lith", hard: true }), filter, 0)).toBe(false);
    expect(isListed(fissure({ era: "axi", hard: false }), filter, 0)).toBe(false);
    expect(isListed(fissure({ era: "axi", hard: true, mission: "MT_SURVIVAL" }), filter, 0)).toBe(
      false,
    );
  });
  it("leaves expired fissures out", () => {
    expect(isListed(fissure({ expires: 10 }), NO_FILTER, 50)).toBe(false);
  });
});

describe("listFissures", () => {
  it("puts the fissure closing soonest first", () => {
    const list = [fissure({ node: "a", expires: 90 }), fissure({ node: "b", expires: 50 })];
    expect(listFissures(list, NO_FILTER, 0).map((each) => each.node)).toEqual(["b", "a"]);
  });
});

describe("watchesFor", () => {
  it("takes one watch per picked era and mission", () => {
    const filter = {
      ...NO_FILTER,
      eras: new Set(["neo"] as const),
      missions: new Set(["MT_SURVIVAL", "MT_DEFENSE"]),
    };
    expect(watchesFor(filter)).toEqual([
      { kind: "fissure", era: "neo", mission: "MT_DEFENSE", of: null },
      { kind: "fissure", era: "neo", mission: "MT_SURVIVAL", of: null },
    ]);
  });
  it("takes one watch per picked era", () => {
    const filter = {
      ...NO_FILTER,
      eras: new Set(["axi", "lith"] as const),
      scope: "hard" as const,
    };
    expect(watchesFor(filter)).toEqual([
      { kind: "fissure", era: "lith", mission: null, of: "hard" },
      { kind: "fissure", era: "axi", mission: null, of: "hard" },
    ]);
  });
  it("takes one watch for every era when none is picked", () => {
    expect(watchesFor({ ...NO_FILTER, missions: new Set(["MT_DEFENSE"]) })).toEqual([
      { kind: "fissure", era: null, mission: "MT_DEFENSE", of: null },
    ]);
    expect(narrows(NO_FILTER)).toBe(false);
  });
});
