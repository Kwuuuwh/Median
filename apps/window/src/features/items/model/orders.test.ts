import { describe, expect, it } from "vitest";
import { firstSort, type Order, pickKey, sortOrders } from "./orders";

const order = (
  name: string,
  status: string,
  platinum: number,
  quantity = 1,
  reputation = 0,
  updated = 0,
): Order => ({
  name,
  reachable: status === "ingame",
  status,
  selling: true,
  platinum,
  quantity,
  perTrade: 1,
  rank: null,
  subtype: null,
  whisper: "",
  reputation,
  updated,
});

describe("orders", () => {
  const rows = [
    order("a", "offline", 10, 1, 0, 300),
    order("b", "ingame", 30, 1, 50, 100),
    order("c", "ingame", 20, 3, 5, 200),
    order("d", "online", 15, 1, 0, 400),
  ];
  const names = (sorted: Order[]) => sorted.map((row) => row.name);

  it("starts sellers in game first, then the cheapest", () => {
    expect(names(sortOrders(rows, firstSort(true)))).toEqual(["c", "b", "d", "a"]);
  });

  it("starts buyers in game first, then the dearest", () => {
    expect(names(sortOrders(rows, firstSort(false)))).toEqual(["b", "c", "d", "a"]);
  });

  it("puts those away first when status ascends", () => {
    expect(names(sortOrders(rows, { status: false }))).toEqual(["a", "d", "c", "b"]);
  });

  it("settles by the fixed order of keys, not the order they were picked", () => {
    const priceFirst = pickKey(pickKey(pickKey({}, "price"), "price"), "status");
    const statusFirst = pickKey(pickKey(pickKey({}, "status"), "price"), "price");
    expect(names(sortOrders(rows, priceFirst))).toEqual(names(sortOrders(rows, statusFirst)));
    expect(names(sortOrders(rows, priceFirst))).toEqual(["a", "d", "b", "c"]);
  });

  it("compares reputation as a number", () => {
    expect(names(sortOrders(rows, { reputation: true }))).toEqual(["b", "c", "d", "a"]);
  });

  it("puts the most recently changed first when nothing sorts", () => {
    expect(names(sortOrders(rows, {}))).toEqual(["d", "a", "c", "b"]);
  });

  it("turns a pressed key on ascending, then descending, then off", () => {
    const once = pickKey({}, "quantity");
    expect(once).toEqual({ quantity: false });
    const twice = pickKey(once, "quantity");
    expect(twice).toEqual({ quantity: true });
    expect(pickKey(twice, "quantity")).toEqual({});
  });
});
