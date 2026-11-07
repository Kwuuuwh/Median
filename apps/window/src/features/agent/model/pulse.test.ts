import type { Agent, Seen } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { paceOf, pulseOf } from "./pulse";

const NOW = 10_000;

function seen(ago: number): Seen {
  return {
    at: NOW - ago,
    item: "/Item",
    name: "Item",
    image: null,
    category: "",
    entry: 10,
    outbid: false,
    worth: 12,
    passed: null,
  };
}

function agent(running: boolean, flow: Seen[]): Agent {
  return {
    running,
    seen: 0,
    taken: 0,
    heard: 0,
    unreachable: 0,
    placed: 0,
    moved: 0,
    warm: 0,
    hot: 0,
    flow,
    picks: [],
  };
}

describe("the agent's pulse", () => {
  it("counts what it weighed a minute over the last five, however long the strip", () => {
    const counts = [
      { at: NOW - 600, seen: 0 },
      { at: NOW - 240, seen: 1_000 },
      { at: NOW, seen: 1_196 },
    ];
    expect(paceOf(counts, NOW)).toBe(49);
  });

  it("says nothing before there are two readings to go by", () => {
    expect(paceOf([{ at: NOW, seen: 10 }], NOW)).toBeNull();
  });

  it("reads off, held back, looking or waiting", () => {
    expect(pulseOf(agent(false, [seen(5)]), [], NOW)).toBe("off");
    expect(pulseOf(agent(true, [seen(5)]), ["purse"], NOW)).toBe("blocked");
    expect(pulseOf(agent(true, [seen(5)]), [], NOW)).toBe("looking");
    expect(pulseOf(agent(true, [seen(600)]), [], NOW)).toBe("waiting");
  });
});
