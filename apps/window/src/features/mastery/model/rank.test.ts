import { describe, expect, it } from "vitest";
import { reached, threshold } from "./rank";

describe("mastery ranks", () => {
  it("ask 2 500 × rank² up to thirty and a flat step past it", () => {
    expect(threshold(20)).toBe(1_000_000);
    expect(threshold(30)).toBe(2_250_000);
    expect(threshold(31)).toBe(2_397_500);
  });

  it("are reached once their threshold is", () => {
    expect(reached(999_999)).toBe(19);
    expect(reached(1_000_000)).toBe(20);
  });
});
