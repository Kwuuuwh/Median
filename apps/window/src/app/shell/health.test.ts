import { type AppState, COLLECTOR_GRACE_S } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { troublesOf } from "./health";

const NOW = 1_700_000_000;

const calm = {
  collector: { state: "live", heard: 0, feeds: { state: "up" } },
  game: { listening: true, shared: false, runningSince: NOW - 600, recognition: "ready" },
  membership: { standing: "in" },
  market: { own: 0, failingSince: null },
  tasks: [{ name: "market", state: { kind: "running" } }],
} as unknown as AppState;

describe("troublesOf", () => {
  it("says nothing while all is well, a closed game without a collector included", () => {
    expect(troublesOf(calm, NOW)).toEqual([]);
    expect(
      troublesOf(
        { ...calm, collector: { state: "offline" }, game: { ...calm.game, runningSince: null } },
        NOW,
      ),
    ).toEqual([]);
  });

  it("names each source that went wrong", () => {
    const troubles = troublesOf(
      {
        ...calm,
        collector: { state: "outdated", speaks: 2 },
        game: { ...calm.game, shared: true },
        market: { own: 0, failingSince: 1_700_000_000 },
        tasks: [{ name: "market", state: { kind: "restarting", reason: "socket closed" } }],
      },
      NOW,
    );
    expect(troubles.map((each) => [each.source, each.said])).toEqual([
      ["gep", "trouble-gep-outdated"],
      ["dbwin", "trouble-dbwin-shared"],
      ["wfm", "trouble-wfm-silent"],
      ["task", "trouble-task-restarting"],
    ]);
    expect(troubles.at(-1)?.reason).toBe("socket closed");
  });

  it("reports the feeds Overwolf refused, with its reason", () => {
    const troubles = troublesOf(
      {
        ...calm,
        collector: {
          state: "live",
          heard: 0,
          feeds: { state: "refused", reason: "not supported" },
        },
      },
      NOW,
    );
    expect(troubles).toEqual([
      { source: "gep", said: "trouble-gep-refused", severe: true, reason: "not supported" },
    ]);
  });

  it("reports a missing collector only once the game has run the grace period without one", () => {
    const started = NOW - COLLECTOR_GRACE_S;
    const offline = {
      ...calm,
      collector: { state: "offline" },
      game: { ...calm.game, runningSince: started },
    } as AppState;

    expect(troublesOf(offline, NOW - 1)).toEqual([]);
    expect(troublesOf(offline, NOW)).toEqual([
      {
        source: "gep",
        said: "trouble-collector-missing",
        severe: true,
        since: started,
        loadsCollector: true,
      },
    ]);
  });

  it("reports reward recognition that did not load", () => {
    const failed = troublesOf({ ...calm, game: { ...calm.game, recognition: "failed" } }, NOW);
    const unchecked = troublesOf(
      { ...calm, game: { ...calm.game, recognition: "unchecked" } },
      NOW,
    );

    expect(failed.map((each) => each.said)).toEqual(["trouble-ocr-failed"]);
    expect(unchecked).toEqual([]);
  });
});
