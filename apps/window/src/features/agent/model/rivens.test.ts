import type { RivenBuy } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { rivenFigures } from "./rivens";

const buy = (
  price: number,
  bought: number,
  sold: number | null,
  sale: number | null,
): RivenBuy => ({
  auction: `${price}:${bought}`,
  name: "crita-visican",
  riven: {
    weapon: "/Lotus/Weapons/ClanTech/Bio/BioWeapon",
    mastery: 8,
    rerolls: 0,
    polarity: null,
    buffs: [],
    curses: [],
  },
  rank: 0,
  price,
  bought,
  sold,
  sale,
});

describe("rivenFigures", () => {
  it("counts buys and sales inside the period and holds what is not sold whenever bought", () => {
    const buys = [buy(300, 100, 400, 450), buy(200, 50, 150, 260), buy(500, 300, null, null)];

    expect(rivenFigures(buys, 120)).toEqual({
      bought: 1,
      spent: 500,
      sold: 2,
      earned: 710,
      profit: 210,
      held: 500,
    });
  });
});
