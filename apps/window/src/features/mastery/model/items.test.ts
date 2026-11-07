import type { Masterable } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { type MasteryFacet, shelfCounts, shownItems } from "./items";

function item(name: string, fields: Partial<Masterable>): Masterable {
  return {
    item: `/${name}`,
    name,
    image: null,
    class: "primary",
    kind: "primary",
    rank: 0,
    cap: 30,
    remaining: 3_000,
    forma: 0,
    passes: 1,
    required: 0,
    owned: false,
    prime: false,
    vaulted: false,
    acquired: null,
    gathered: 0,
    inputs: 0,
    lacking: [],
    stage: "obtain",
    way: null,
    ...fields,
  };
}

const items = [
  item("Dropped", {
    way: {
      kind: "drop",
      place: "Ceres",
      area: null,
      enemy: false,
      chance: 100,
      planet: null,
      mission: null,
      levelFrom: null,
      levelTo: null,
      haunts: [],
    },
  }),
  item("Bought", { way: { kind: "credits", vendor: "Market", credits: 25_000 } }),
  item("Unknown", {}),
  item("Held", { stage: "level", owned: true, remaining: 1_500 }),
  item("HeldRicher", { stage: "level", owned: true, remaining: 6_000 }),
  item("Short2", { stage: "short", gathered: 1, inputs: 3 }),
  item("Short1", { stage: "short", gathered: 2, inputs: 3 }),
  item("Done", { stage: "mastered", remaining: 0 }),
  item("Frame", { class: "warframe", kind: "warframe", remaining: 6_000 }),
];

const every = {
  shelf: null,
  search: () => true,
  facets: new Map<MasteryFacet, boolean>(),
  planned: new Set(["/Held", "/Frame"]),
};

describe("the mastery items", () => {
  it("read by name, mastered ones left out", () => {
    expect(shownItems(items, 10, every).map((each) => each.name)).toEqual([
      "Bought",
      "Dropped",
      "Frame",
      "Held",
      "HeldRicher",
      "Short1",
      "Short2",
      "Unknown",
    ]);
  });

  it("show only the mastered ones when asked for them", () => {
    const facets = new Map<MasteryFacet, boolean>([["mastered", true]]);

    expect(shownItems(items, 10, { ...every, facets }).map((each) => each.name)).toEqual(["Done"]);
  });

  it("show only what is in the plan when asked for it", () => {
    const facets = new Map<MasteryFacet, boolean>([["planned", true]]);

    expect(shownItems(items, 10, { ...every, facets }).map((each) => each.name)).toEqual([
      "Frame",
      "Held",
    ]);
  });

  it("count what is still to master on each shelf", () => {
    const counts = shelfCounts(items);

    expect(counts.get("primary")).toBe(7);
    expect(counts.get("warframe")).toBe(1);
  });
});
