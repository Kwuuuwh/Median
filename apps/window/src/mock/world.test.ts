import type { Worldstate } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { worldAsOf } from "./world";

describe("worldAsOf", () => {
  it("moves every clock by the time since the world was recorded", () => {
    const world: Worldstate = {
      at: 100,
      cycles: [{ key: "cetus", phases: [{ key: "day", seconds: 10 }], phase: 0, ends: 150 }],
      fissures: [
        { node: "SolNode1", era: "lith", hard: false, storm: false, mission: null, expires: 400 },
      ],
      deadlines: [{ reset: "daily", at: 200 }],
      visits: [{ vendor: "baro", node: "SaturnHUB", arrives: 300, departs: 500 }],
    };
    const moved = worldAsOf(world, 1100);
    expect(moved.at).toBe(1100);
    expect(moved.cycles[0].ends).toBe(1150);
    expect(moved.fissures[0].expires).toBe(1400);
    expect(moved.deadlines[0].at).toBe(1200);
    expect(moved.visits[0]).toMatchObject({ arrives: 1300, departs: 1500 });
  });
});
