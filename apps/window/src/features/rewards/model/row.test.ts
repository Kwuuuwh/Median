import { describe, expect, it } from "vitest";
import { row } from "./row";

/**
 * Screen width and card step in pixels at which the reward reader measures the name plates;
 * matches crates/median-app/src/reward/cards.rs.
 */
const MEASURED = 1920;
const STEP = 243;

function left(cards: number): number {
  return (Number.parseFloat(String(row(cards).left)) / 100) * MEASURED;
}

describe("the row of cards", () => {
  it("stands where a full squad was measured", () => {
    expect(left(4)).toBeCloseTo(478);
    expect(row(4).width).toBe(`${((STEP * 4) / MEASURED) * 100}%`);
  });

  it("keeps its middle when a card is missing", () => {
    expect(left(4) + (STEP * 4) / 2).toBeCloseTo(left(3) + (STEP * 3) / 2);
    expect(left(2) + (STEP * 2) / 2).toBeCloseTo(left(1) + STEP / 2);
  });

  it("holds a squad of two where a full squad holds its second and third", () => {
    expect(left(2)).toBeCloseTo(left(4) + STEP);
  });

  it("gives the panel a column for each card and no more", () => {
    expect(row(3).gridTemplateColumns).toBe("repeat(3, 1fr)");
  });
});
