import { describe, expect, it } from "vitest";
import { DAY } from "@/shared/lib/time";
import { dailyTrades, daysOf, lastPriced, layersOf, nearestDay, priceTicks } from "./chart";

const empty = { closed: [], selling: [], buying: [] };

describe("chart", () => {
  const NOW = 100 * DAY + 3_600;

  it("lays a whole calendar of days, those without trades empty", () => {
    const days = daysOf({ ...empty, closed: [{ at: 98 * DAY, platinum: 10, weight: 4 }] }, 3, NOW);
    expect(days).toEqual([
      { at: 98 * DAY, platinum: 10, trades: 4, selling: null, buying: null },
      { at: 99 * DAY, platinum: null, trades: 0, selling: null, buying: null },
      { at: 100 * DAY, platinum: null, trades: 0, selling: null, buying: null },
    ]);
    expect(lastPriced(days, ["trades"])).toBe(0);
    expect(layersOf(days)).toEqual(["trades"]);
  });

  it("lays each side on the day it stood and leaves out what is older", () => {
    const days = daysOf(
      {
        closed: [{ at: 99 * DAY, platinum: 10, weight: 4 }],
        selling: [
          { at: 90 * DAY, best: 20, median: 25, orders: 30 },
          { at: 99 * DAY + 60, best: 12, median: 15, orders: 30 },
          { at: 100 * DAY, best: 11, median: 14, orders: 28 },
        ],
        buying: [{ at: 100 * DAY, best: 8, median: 6, orders: 9 }],
      },
      2,
      NOW,
    );
    expect(days).toEqual([
      { at: 99 * DAY, platinum: 10, trades: 4, selling: 12, buying: null },
      { at: 100 * DAY, platinum: null, trades: 0, selling: 11, buying: 8 },
    ]);
    expect(lastPriced(days, ["trades"])).toBe(0);
    expect(lastPriced(days, ["selling", "buying"])).toBe(1);
    expect(layersOf(days)).toEqual(["selling", "buying"]);
  });

  it("counts trades a day over the whole week before today", () => {
    const closed = [92, 94, 99, 100].map((day) => ({ at: day * DAY, platinum: 5, weight: 7 }));
    expect(dailyTrades({ closed }, NOW)).toBe(2);
    expect(dailyTrades({ closed: [] }, NOW)).toBeNull();
  });

  it("steps a price axis by round numbers", () => {
    expect(priceTicks(38, 112)).toEqual([40, 60, 80, 100]);
    expect(priceTicks(4, 9)).toEqual([4, 6, 8]);
    expect(priceTicks(4.4, 6.6)).toEqual([5, 6]);
  });

  it("finds the day under a point of the chart", () => {
    const days = daysOf(
      {
        ...empty,
        closed: [1, 2, 3, 4, 5].map((at) => ({ at: at * DAY, platinum: at, weight: 1 })),
      },
      5,
      5 * DAY,
    );
    expect(nearestDay(days, 0.6)).toBe(2);
    expect(nearestDay(days, 1.4)).toBe(4);
  });
});
