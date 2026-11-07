import { describe, expect, it } from "vitest";
import { type Facets, passes } from "./facets";
import type { HeldLine } from "./lines";

const line = (over: Partial<HeldLine>): HeldLine => ({
  item: "/A",
  variant: "",
  name: "A",
  image: null,
  known: true,
  kind: "",
  group: "",
  count: 1,
  detail: null,
  rank: null,
  maxRank: null,
  fitted: [],
  equipped: false,
  tradable: true,
  prime: false,
  vaulted: false,
  ducats: null,
  built: null,
  platinum: null,
  topped: null,
  toMax: null,
  spare: 0,
  collection: null,
  pair: null,
  ...over,
});

const marks = { listed: (each: HeldLine) => each.item === "/Listed", favourite: () => false };

describe("passes", () => {
  it("lets through lines that answer every facet as wanted", () => {
    const facets: Facets = new Map([
      ["many", true],
      ["listed", false],
    ]);
    expect(passes(line({ count: 3 }), facets, marks)).toBe(true);
    expect(passes(line({ count: 1 }), facets, marks)).toBe(false);
    expect(passes(line({ count: 3, item: "/Listed" }), facets, marks)).toBe(false);
  });

  it("reads an upgrade from the rank", () => {
    const facets: Facets = new Map([["upgraded", false]]);
    expect(passes(line({ rank: 0 }), facets, marks)).toBe(true);
    expect(passes(line({ rank: 3 }), facets, marks)).toBe(false);
  });
});
