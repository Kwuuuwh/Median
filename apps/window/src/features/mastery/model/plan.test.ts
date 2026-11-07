import type { Masterable, Planned, Requirement, Way } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { byGet, levelling, needCounts, needLines } from "./plan";

function line(name: string, fields: Partial<Requirement>): Requirement {
  return {
    item: `/${name}`,
    name,
    image: null,
    kind: "resource",
    count: 1,
    have: 0,
    forging: 0,
    built: false,
    tradable: false,
    slug: null,
    price: null,
    buy: false,
    ownedRelics: 0,
    ownedChance: 0,
    relics: 0,
    relicNames: [],
    vaulted: false,
    way: null,
    ways: [],
    runs: null,
    areas: [],
    ...fields,
  };
}

const drop = (place: string): Way => ({
  kind: "drop",
  place,
  area: null,
  enemy: false,
  chance: 100,
  planet: null,
  mission: null,
  levelFrom: null,
  levelTo: null,
  haunts: [],
});

const lines = [
  line("Held", { have: 1 }),
  line("Rubedo", { count: 500, have: 100 }),
  line("Plastids", { count: 400, have: 380, areas: ["fortuna"] }),
  line("Neuroptics", { kind: "warframe-component", built: true }),
  line("Chassis", { kind: "warframe-component", built: true }),
  line("Prime Blueprint", { kind: "blueprint", relics: 4, relicNames: ["Lith A1"] }),
  line("Prime Barrel", { kind: "weapon-component", ownedRelics: 2, relicNames: ["Meso B2"] }),
  line("Lens", { kind: "blueprint", way: drop("Earth") }),
];

describe("the plan", () => {
  it("counts what each need still waits for, what is held left out", () => {
    expect(Object.fromEntries(needCounts(lines, null, 3))).toEqual({
      level: 3,
      resources: 2,
      forge: 2,
      relics: 2,
      obtain: 1,
    });
  });

  it("puts the most short resource first", () => {
    expect(needLines(lines, "resources", null, new Set()).map((each) => each.name)).toEqual([
      "Rubedo",
      "Plastids",
    ]);
  });

  it("puts what the foundry can start now first", () => {
    const startable = new Set(["/Neuroptics"]);

    expect(needLines(lines, "forge", null, startable).map((each) => each.name)).toEqual([
      "Neuroptics",
      "Chassis",
    ]);
  });

  it("puts parts with relics held first", () => {
    expect(needLines(lines, "relics", null, new Set()).map((each) => each.name)).toEqual([
      "Prime Barrel",
      "Prime Blueprint",
    ]);
  });

  it("files a vaulted part under relics, even with no relic of it held", () => {
    const vaulted = line("Vaulted Part", { kind: "weapon-component", vaulted: true });

    expect(needLines([vaulted], "relics", null, new Set())).toHaveLength(1);
  });

  it("splits what is to get by how it is got, credits first", () => {
    const bought = line("Bought", {
      kind: "blueprint",
      way: { kind: "credits", vendor: "Market", credits: 25_000 },
    });

    expect(byGet([...lines, bought]).map((group) => group.get)).toEqual([
      "credits",
      "drop",
      "unknown",
    ]);
  });

  it("keeps to what is got in the picked area", () => {
    expect(needLines(lines, "resources", "fortuna", new Set()).map((each) => each.name)).toEqual([
      "Plastids",
    ]);
  });

  it("levels what is held, the nearest to its cap first, what is at its cap left out", () => {
    const planned = (item: string, owned: boolean): Planned => ({
      item,
      name: item,
      image: null,
      owned,
      remaining: 1,
    });
    const masterable = (item: string, rank: number, cap: number) =>
      ({ item, rank, cap }) as Masterable;

    const levelled = levelling(
      [planned("A", true), planned("B", true), planned("C", false), planned("D", true)],
      [masterable("A", 10, 30), masterable("B", 25, 30), masterable("D", 30, 30)],
    );

    expect(levelled.map((each) => each.planned.item)).toEqual(["B", "A"]);
  });
});
