import { describe, expect, it } from "vitest";
import { scaleOf } from "./stage";

describe("the overlay's scale", () => {
  it("is one on the screen it was laid out on", () => {
    expect(scaleOf(1920, 1080)).toBe(1);
  });

  it("follows the narrower side of a screen of another shape", () => {
    expect(scaleOf(2560, 1440)).toBeCloseTo(4 / 3);
    expect(scaleOf(3440, 1440)).toBeCloseTo(4 / 3);
  });
});
