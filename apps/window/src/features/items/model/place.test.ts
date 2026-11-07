import { describe, expect, it } from "vitest";
import { type DropWay, levelsOf, placeText } from "./place";

const way = (fields: Partial<DropWay>): DropWay => ({
  kind: "drop",
  place: "",
  area: null,
  enemy: false,
  chance: 100,
  planet: null,
  mission: null,
  levelFrom: null,
  levelTo: null,
  haunts: [],
  ...fields,
});

describe("a drop's place", () => {
  it("reads from the planet down to the mission", () => {
    expect(placeText(way({ planet: "Уран", place: "Титания", mission: "Убийство" }))).toBe(
      "Уран · Титания · Убийство",
    );
  });

  it("says a word once, keeping the fuller one", () => {
    expect(placeText(way({ place: "Цетус", mission: "Заказы Цетуса" }))).toBe("Заказы Цетуса");
  });

  it("names the planets an enemy is met on, the rest counted", () => {
    expect(
      placeText(way({ enemy: true, place: "Лансер", haunts: ["Марс", "Церера", "Земля"] })),
    ).toBe("Лансер · Марс, Церера +1");
    expect(placeText(way({ enemy: true, place: "Лансер" }))).toBe("Лансер");
  });

  it("gives the levels a place runs at", () => {
    expect(levelsOf(way({ levelFrom: 10, levelTo: 30 }))).toBe("10–30");
    expect(levelsOf(way({}))).toBeNull();
  });
});
