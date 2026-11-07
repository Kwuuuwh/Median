import { describe, expect, it } from "vitest";
import { elementPicture } from "./element";

describe("elementPicture", () => {
  it("draws the damage types and nothing else", () => {
    expect(elementPicture("WeaponToxinDamageMod")).toBe("/statuses/toxin.webp");
    expect(elementPicture("WeaponArmorPiercingDamageMod")).toBe("/statuses/puncture.webp");
    expect(elementPicture("WeaponCritChanceMod")).toBeNull();
  });
});
