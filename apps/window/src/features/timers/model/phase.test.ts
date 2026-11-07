import type { Cycle } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { currentPhase } from "./phase";

const cetus: Cycle = {
  key: "cetus",
  phases: [
    { key: "day", seconds: 6000 },
    { key: "night", seconds: 3000 },
  ],
  phase: 0,
  ends: 1000,
};

describe("currentPhase", () => {
  it("keeps the reported phase while it runs", () => {
    expect(currentPhase(cetus, 400)).toEqual({ phase: "day", ends: 1000, next: "night" });
  });

  it("carries on past a reported phase that has already ended", () => {
    expect(currentPhase(cetus, 1500)).toEqual({ phase: "night", ends: 4000, next: "day" });
  });

  it("wraps around a long gap since the report", () => {
    expect(currentPhase(cetus, 1000 + 9000 * 3 + 10)?.phase).toBe("night");
  });
});
