import type { AppState, Catalog, Game, Piece, Quote, Swap } from "@median/ipc";
import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { applyMoves, dataReplaced, followEvents, quotesFrom, traded } from "./events";

const followed = vi.hoisted(() => new Set<string>());

vi.mock("@median/ipc", async (original) => {
  const ipc = await original<typeof import("@median/ipc")>();
  const events = Object.fromEntries(
    Object.keys(ipc.events).map((name) => [
      name,
      {
        listen: async () => {
          followed.add(name);
          return () => {};
        },
      },
    ]),
  );
  return { ...ipc, events };
});

describe("followEvents", () => {
  it("follows every event the backend emits", async () => {
    const ipc = await vi.importActual<typeof import("@median/ipc")>("@median/ipc");
    /** Followed outside `followEvents`: by the updater and the alarm listener. */
    const elsewhere = ["updateProgressed", "rang"];
    await followEvents(new QueryClient());
    expect([...followed].sort()).toEqual(
      Object.keys(ipc.events)
        .filter((name) => !elsewhere.includes(name))
        .sort(),
    );
  });
});

const piece = (item: string, count: number): Piece => ({
  item,
  variant: "",
  count,
  detail: null,
  equipped: false,
});

describe("applyMoves", () => {
  it("replaces moved lines and drops those at zero", () => {
    const after = applyMoves(
      { step: 1, digest: "a", pieces: [piece("/A", 1), piece("/B", 2)] },
      2,
      "b",
      [piece("/A", 5), piece("/B", 0), piece("/C", 1)],
    );
    expect(after).toEqual({ step: 2, digest: "b", pieces: [piece("/A", 5), piece("/C", 1)] });
  });
});

describe("traded", () => {
  const swap = (at: number) => ({ at }) as Swap;
  const game = (...swaps: Swap[]) => ({ swaps }) as Game;
  const state = (...swaps: Swap[]) => ({ game: game(...swaps) }) as AppState;

  it("takes a newest exchange the state did not have as a trade", () => {
    expect(traded(state(swap(1)), game(swap(2), swap(1)))).toBe(true);
    expect(traded(undefined, game(swap(2)))).toBe(true);
  });

  it("takes anything else the game said as no trade", () => {
    expect(traded(state(swap(2)), game(swap(2)))).toBe(false);
    expect(traded(state(), game())).toBe(false);
  });
});

describe("dataReplaced", () => {
  const catalog = (installed: string | null, pack: string | null): Catalog => ({
    ready: installed !== null && pack !== null,
    installed,
    pack,
    pending: null,
    activity: { state: "idle" },
  });
  const stateWith = (installed: string | null, pack: string | null) =>
    ({ catalog: catalog(installed, pack) }) as AppState;

  it("is true when data in use changes version", () => {
    expect(dataReplaced(stateWith("1.2.0", "1.2.0"), catalog("1.3.0", "1.2.0"))).toBe(true);
    expect(dataReplaced(stateWith("1.3.0", "1.2.0"), catalog("1.3.0", "1.3.0"))).toBe(true);
  });

  it("is false for the first install and for a state that did not change", () => {
    expect(dataReplaced(stateWith(null, null), catalog("1.2.0", "1.2.0"))).toBe(false);
    expect(dataReplaced(undefined, catalog("1.2.0", "1.2.0"))).toBe(false);
    expect(dataReplaced(stateWith("1.2.0", "1.2.0"), catalog("1.2.0", "1.2.0"))).toBe(false);
  });
});

describe("quotesFrom", () => {
  const quote = (item: string, slug: string | null): Quote => ({
    item,
    platinum: null,
    pair: slug === null ? null : { slug, key: "" },
  });

  it("finds a quote priced from one of the slugs", () => {
    const quotes = [quote("/Mod", "primed_flow"), quote("/Part", "braton_prime_barrel")];
    expect(quotesFrom(quotes, new Set(["primed_flow"]))).toBe(true);
    expect(quotesFrom(quotes, new Set(["lex_prime_set"]))).toBe(false);
  });

  it("finds nothing in quotes without a pair or not loaded yet", () => {
    expect(quotesFrom([quote("/Skin", null)], new Set(["primed_flow"]))).toBe(false);
    expect(quotesFrom(undefined, new Set(["primed_flow"]))).toBe(false);
  });
});
