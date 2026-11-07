import type { OwnOrder } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { gapOf, sortBook, standingOf } from "./book";

const order = (over: Partial<OwnOrder>): OwnOrder => ({
  id: "1",
  item: "/A",
  slug: "a",
  name: "A",
  image: null,
  variant: "",
  maxRank: null,
  selling: true,
  platinum: 20,
  quantity: 1,
  owned: null,
  perTrade: 1,
  visible: true,
  byAgent: false,
  reference: 18,
  basis: null,
  ours: null,
  most: null,
  resale: null,
  ...over,
});

describe("standing", () => {
  it("reads a seller beaten by a cheaper one and a buyer by a dearer one", () => {
    expect(standingOf(order({}))).toBe("beaten");
    expect(gapOf(order({}))).toBe(2);
    expect(standingOf(order({ platinum: 18 }))).toBe("best");
    expect(standingOf(order({ selling: false, platinum: 10, reference: 12 }))).toBe("beaten");
    expect(standingOf(order({ reference: null }))).toBe("unknown");
  });
});

describe("sortBook", () => {
  it("puts the widest gap first and unknown ones last", () => {
    const orders = [
      order({ id: "a", name: "А", reference: null }),
      order({ id: "b", name: "Б", reference: 19 }),
      order({ id: "c", name: "В", reference: 10 }),
    ];
    const ids = (descending: boolean) =>
      sortBook(orders, { by: "gap", descending }, "ru").map((each) => each.id);
    expect(ids(true)).toEqual(["c", "b", "a"]);
    expect(ids(false)).toEqual(["b", "c", "a"]);
  });

  it("turns names round when asked", () => {
    const orders = [order({ id: "a", name: "А" }), order({ id: "b", name: "Б" })];
    expect(sortBook(orders, { by: "name", descending: true }, "ru").map((each) => each.id)).toEqual(
      ["b", "a"],
    );
  });
});
