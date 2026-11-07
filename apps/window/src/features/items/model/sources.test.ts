import type { Reward } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { relicSources, runsFor } from "./sources";

const reward = (item: string, name: string, refinement: string, chance: number): Reward => ({
  item,
  name,
  image: null,
  rarity: "uncommon",
  refinement,
  chance,
  count: 1,
});

describe("relicSources", () => {
  const found = [
    reward("/A/Intact", "Лит A1", "intact", 110),
    reward("/A/Radiant", "Лит A1", "radiant", 200),
    reward("/B/Intact", "Мезо B2", "intact", 253),
    reward("/C/Intact", "Нео C3", "intact", 20),
  ];

  it("folds refinements into one relic and puts held, then dropping, then likeliest first", () => {
    const sources = relicSources(
      found,
      (item) => (item === "/C/Intact" ? 2 : 0),
      (item) => item === "/B/Intact",
      "intact",
    );
    expect(sources.map((source) => source.name)).toEqual(["Нео C3", "Лит A1", "Мезо B2"]);
    expect(sources[1].chance).toEqual({ intact: 110, radiant: 200 });
    expect(sources[1].items).toEqual(["/A/Intact", "/A/Radiant"]);
  });

  it("opens a relic on its intact refinement, whichever refinement comes first", () => {
    const [relic] = relicSources(
      [
        reward("/A/Radiant", "Лит A1", "radiant", 200),
        reward("/A/Intact", "Лит A1", "intact", 110),
      ],
      () => 0,
      () => false,
      "intact",
    );
    expect(relic.opens).toBe("/A/Intact");
  });

  it("keeps a vaulted relic a trader sells among those to be had", () => {
    const [first] = relicSources(
      found,
      () => 0,
      () => true,
      "intact",
      (item) => item === "/C/Intact",
    );
    expect(first).toMatchObject({ name: "Нео C3", offered: true });
  });

  it("counts runs a chance takes on average", () => {
    expect(runsFor(253)).toBe(4);
    expect(runsFor(20)).toBe(50);
  });
});
