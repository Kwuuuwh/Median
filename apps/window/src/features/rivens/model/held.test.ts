import type { Piece } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { classOf, heldRivens } from "./held";

const riven = (item: string, revealed: boolean, count = 1): Piece => ({
  item,
  variant: revealed ? '{"compat":"/Gun"}' : "",
  count,
  equipped: false,
  detail: {
    kind: "riven",
    rank: 3,
    revealed: revealed
      ? { weapon: "/Gun", mastery: 9, rerolls: 4, polarity: "madurai", buffs: [], curses: [] }
      : null,
  },
});

describe("classOf", () => {
  it("reads the class a riven item key names", () => {
    expect(classOf("/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare")).toBe("rifle");
    expect(classOf("/Lotus/Upgrades/Mods/Randomized/RawModularPistolRandomMod")).toBe("kitgun");
    expect(classOf("/Lotus/Upgrades/Mods/Randomized/PlayerMeleeWeaponRandomModRare")).toBe("melee");
    expect(classOf("/Lotus/Upgrades/Mods/Rifle/WeaponDamageAmountMod")).toBeNull();
  });
});

describe("heldRivens", () => {
  it("keeps revealed rivens one by one and counts veiled ones by class", () => {
    const held = heldRivens([
      riven("/Lotus/Upgrades/Mods/Randomized/LotusRifleRandomModRare", true),
      riven("/Lotus/Upgrades/Mods/Randomized/RawRifleRandomMod", false, 3),
      riven("/Lotus/Upgrades/Mods/Randomized/RawPistolRandomMod", false, 0),
      riven("/Lotus/Upgrades/Mods/Randomized/RawShotgunRandomMod", false, 2),
    ]);

    expect(held.revealed.map((each) => [each.class, each.rank])).toEqual([["rifle", 3]]);
    expect(held.veiled).toEqual([
      { class: "rifle", count: 3 },
      { class: "shotgun", count: 2 },
    ]);
  });
});
