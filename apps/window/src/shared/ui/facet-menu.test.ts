import { describe, expect, it } from "vitest";
import { type FacetRange, rangeWord } from "./facet-menu";

const range = (from: string, to: string): FacetRange => ({
  label: "Платина",
  from,
  to,
  invalidFrom: false,
  invalidTo: false,
  onChange: () => undefined,
});

describe("rangeWord", () => {
  it("names both bounds, or the one side typed", () => {
    expect(rangeWord(range("10", "50"), "от", "до")).toBe("Платина 10–50");
    expect(rangeWord(range("10", ""), "от", "до")).toBe("Платина от 10");
    expect(rangeWord(range("", "50"), "от", "до")).toBe("Платина до 50");
  });

  it("says nothing for an open range", () => {
    expect(rangeWord(range("", ""), "от", "до")).toBeNull();
  });
});
