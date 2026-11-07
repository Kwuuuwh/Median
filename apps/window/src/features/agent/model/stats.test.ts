import type { Dealt, Position } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { DAY } from "@/shared/lib/time";
import { byAgent, byGroup, daily, earners, markup, summarize, unsold } from "./stats";

const NOW = 100 * DAY + 3_600;

function deal(
  name: string,
  selling: boolean,
  platinum: number,
  at: number,
  margin: number | null = null,
  agent = true,
  group = "mod",
): Dealt {
  return {
    at,
    item: `/Lotus/${name}`,
    name,
    image: null,
    selling,
    platinum,
    quantity: 1,
    agent,
    group,
    margin,
  };
}

const deals = [
  deal("Rubico", true, 55, NOW - 100, 15),
  deal("Rubico", false, 40, NOW - DAY),
  deal("Galatine", true, 15, NOW - 2 * DAY, -5),
  deal("Voruna", true, 30, NOW - 2 * DAY, null, false, "prime"),
];

describe("the trading figures", () => {
  it("count sales, purchases and what the sales made apart", () => {
    expect(summarize(deals)).toEqual({
      margin: 10,
      margined: 2,
      taken: 100,
      sales: 3,
      spent: 40,
      buys: 1,
    });
  });

  it("take the exchanges that filled the agent's orders", () => {
    expect(byAgent(deals).map((each) => each.name)).toEqual(["Rubico", "Rubico", "Galatine"]);
  });

  it("lay sales and purchases out day by day, a quiet day at zero", () => {
    const series = daily(deals, 4, NOW);

    expect(series.map((day) => [day.taken, day.spent, day.margin])).toEqual([
      [0, 0, 0],
      [45, 0, -5],
      [0, 40, 0],
      [55, 0, 15],
    ]);
  });

  it("rank items by what their sales made, leaving out sales with no cost to count", () => {
    expect(earners(deals, 5).map((each) => [each.name, each.margin])).toEqual([
      ["Rubico", 15],
      ["Galatine", -5],
    ]);
  });

  it("break the figures down by every group of goods, the one that made most first", () => {
    expect(byGroup(deals).map((each) => [each.group, each.margin, each.taken, each.spent])).toEqual(
      [
        ["mod", 10, 70, 40],
        ["prime", 0, 30, 0],
        ["arcane", 0, 0, 0],
        ["set", 0, 0, 0],
        ["relic", 0, 0, 0],
        ["riven", 0, 0, 0],
        ["other", 0, 0, 0],
      ],
    );
  });

  it("weigh what the margined sales made against what their copies cost", () => {
    expect(markup(deals)).toBe(17);
    expect(markup([])).toBeNull();
  });

  it("add up the copies bought and not sold, priced at the cheapest seller or their cost", () => {
    const position = (basis: number, quantity: number, ingame: number | null) =>
      ({ basis, quantity, ingame }) as Position;

    expect(unsold([position(30, 2, 40), position(10, 1, null)])).toEqual({
      cost: 70,
      worth: 90,
      copies: 3,
    });
  });
});
