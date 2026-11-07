import type { Alarm } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { currentFeed, markSeen, recordAlarms } from "./feed";

const night: Alarm = { watch: { kind: "phase", cycle: "cetus", phase: "night" }, node: null };
const baro: Alarm = { watch: { kind: "arrival", vendor: "baro" }, node: "SaturnHUB" };

describe("feed", () => {
  it("keeps the newest first and counts what was not seen", () => {
    recordAlarms([night, baro], 10);
    markSeen();
    recordAlarms([night], 20);

    const { rung, unseen } = currentFeed();
    expect(rung.map((each) => each.at)).toEqual([20, 10, 10]);
    expect(rung[1].alarm).toEqual(baro);
    expect(unseen).toBe(1);
  });
});
