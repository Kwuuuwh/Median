import { describe, expect, it } from "vitest";
import { gradeRank, sellsBetter } from "./grade";

describe("sellsBetter", () => {
  it("weighs the grade before the price", () => {
    expect(sellsBetter({ grade: "good", price: 100 }, { grade: "bad", price: 900 })).toBe(true);
    expect(sellsBetter({ grade: "bad", price: 900 }, { grade: "perfect", price: 100 })).toBe(false);
  });

  it("falls back to the price on equal or unknown grades", () => {
    expect(sellsBetter({ grade: "good", price: 300 }, { grade: "good", price: 200 })).toBe(true);
    expect(sellsBetter({ grade: null, price: 100 }, { grade: "good", price: 200 })).toBe(false);
    expect(sellsBetter({ grade: null, price: null }, { grade: "good", price: 200 })).toBeNull();
  });

  it("sorts an unknown grade last", () => {
    expect(gradeRank(null)).toBeLessThan(gradeRank("bad"));
  });
});
