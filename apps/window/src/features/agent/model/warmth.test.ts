import { describe, expect, it } from "vitest";
import { warmthOf } from "./warmth";

describe("warmthOf", () => {
  it("counts the map before anything is warm", () => {
    expect(warmthOf({ warm: 0, hot: 0 })).toEqual({ phase: "counting" });
  });

  it("is warming until every watched item has been asked about", () => {
    expect(warmthOf({ warm: 100, hot: 400 })).toEqual({
      phase: "warming",
      warm: 100,
      hot: 400,
      share: 0.25,
    });
  });

  it("stays warm when a recount watches fewer items than were asked about", () => {
    expect(warmthOf({ warm: 313, hot: 300 })).toMatchObject({ phase: "warm", warm: 300 });
  });
});
