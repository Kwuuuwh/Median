import type { Collectible } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { type Narrowed, shelve } from "./shelf";

function row(fields: Partial<Collectible> & { item: string }): Collectible {
  return {
    name: fields.item,
    image: null,
    class: "primary",
    kind: "primary",
    prime: false,
    vaulted: false,
    tradable: false,
    owned: 0,
    mastered: null,
    ready: null,
    required: 0,
    reachable: null,
    relics: false,
    ...fields,
  };
}

function narrowed(change: Partial<Narrowed>): Narrowed {
  return {
    className: null,
    matches: () => true,
    facets: new Map(),
    favourite: () => false,
    locale: "en",
    ...change,
  };
}

const VOLT = row({ item: "Volt", class: "warframe", owned: 1, mastered: true });
const GARA = row({ item: "Gara", class: "warframe", ready: true, relics: true });
const VITALITY = row({ item: "Vitality", class: "mod" });

const ROWS = [VOLT, VITALITY, GARA];

describe("shelve", () => {
  it("lists everything by name while nothing is held back", () => {
    expect(shelve(ROWS, narrowed({}))).toEqual([GARA, VITALITY, VOLT]);
  });

  it("keeps to the class it was asked for", () => {
    expect(shelve(ROWS, narrowed({ className: "warframe" }))).toEqual([GARA, VOLT]);
  });

  it("keeps to what the search lets through", () => {
    expect(shelve(ROWS, narrowed({ matches: (_, name) => name.startsWith("V") }))).toEqual([
      VITALITY,
      VOLT,
    ]);
  });

  it("lets through what is ready and holds back what cannot say", () => {
    expect(shelve(ROWS, narrowed({ facets: new Map([["ready", true]]) }))).toEqual([GARA]);
    expect(shelve(ROWS, narrowed({ facets: new Map([["ready", false]]) }))).toEqual([]);
  });

  it("asks the favourites about the item", () => {
    const marked = narrowed({
      facets: new Map([["favourite", true]]),
      favourite: (item) => item === "Vitality",
    });
    expect(shelve(ROWS, marked)).toEqual([VITALITY]);
  });

  it("answers what the account holds from its copies", () => {
    expect(shelve(ROWS, narrowed({ facets: new Map([["held", false]]) }))).toEqual([
      GARA,
      VITALITY,
    ]);
  });

  it("needs every active filter to agree", () => {
    const both = narrowed({
      facets: new Map([
        ["relics", true],
        ["held", true],
      ]),
    });
    expect(shelve(ROWS, both)).toEqual([]);
  });
});
