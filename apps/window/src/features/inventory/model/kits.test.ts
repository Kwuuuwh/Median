import type { Good, SetRow } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { kitsOf, missingCost, missingOf, sortKits } from "./kits";

const zhuge: SetRow = {
  slug: "zhuge_prime_set",
  name: "Zhuge Prime Set",
  ducats: null,
  vaulted: true,
  built: "/Zhuge",
  members: ["/Zhuge/Bp", "/Zhuge/Barrel", "/Zhuge/Grip"],
  per_set: [1, 1, 1],
};
const braton: SetRow = {
  slug: "braton_prime_set",
  name: "Braton Prime Set",
  ducats: null,
  vaulted: false,
  built: "/Braton",
  members: ["/Braton/Bp", "/Braton/Barrel"],
  per_set: [1, 1],
};
const lonely: SetRow = {
  ...braton,
  slug: "lonely_set",
  built: "/Lonely",
  members: ["/Lonely/Bp"],
  per_set: [1],
};

const names: Record<string, string> = {
  "/Zhuge": "Чжугэ Прайм",
  "/Zhuge/Bp": "Чжугэ Прайм (Чертеж)",
  "/Zhuge/Barrel": "Чжугэ Прайм: Ствол",
  "/Zhuge/Grip": "Чжугэ Прайм: Рукоять",
};
const good = (item: string) =>
  names[item] ? ({ item, name: names[item], image: null, ducats: 15 } as Good) : undefined;
const held = new Map([
  ["/Zhuge/Bp", 1],
  ["/Zhuge/Barrel", 2],
  ["/Braton/Bp", 1],
  ["/Braton/Barrel", 1],
]);
const prices: Record<string, number> = { "/Zhuge/Grip": 6, "/Braton/Barrel": 4 };

const kits = kitsOf(
  [zhuge, braton, lonely],
  held,
  good,
  (item) => prices[item] ?? null,
  (slug) => (slug === "zhuge_prime_set" ? 49 : 20),
);

describe("the kits", () => {
  it("take in every set any part of which is held, and none of the rest", () => {
    expect(kits.map((kit) => kit.slug)).toEqual(["zhuge_prime_set", "braton_prime_set"]);
  });

  it("name each part without its set and count the whole ones the parts make", () => {
    const [zhugeKit, bratonKit] = kits;
    expect(zhugeKit.parts.map((part) => part.name)).toEqual(["Чертеж", "Ствол", "Рукоять"]);
    expect(zhugeKit.whole).toBe(0);
    expect(bratonKit.whole).toBe(1);
  });

  it("say what is missing and what it costs to buy", () => {
    expect(missingOf(kits[0]).map((part) => part.item)).toEqual(["/Zhuge/Grip"]);
    expect(missingCost(kits[0])).toBe(6);
    expect(missingCost(kits[1])).toBe(0);
  });

  it("count a part a set takes twice as missing until both are held", () => {
    const zoren: SetRow = {
      ...braton,
      slug: "dual_zoren_prime_set",
      built: "/Zoren",
      members: ["/Zoren/Bp", "/Zoren/Blade"],
      per_set: [1, 2],
    };
    const [kit] = kitsOf(
      [zoren],
      new Map([
        ["/Zoren/Bp", 1],
        ["/Zoren/Blade", 1],
      ]),
      good,
      () => 5,
      () => 20,
    );
    expect(kit.whole).toBe(0);
    expect(missingOf(kit).map((part) => part.item)).toEqual(["/Zoren/Blade"]);
    expect(missingCost(kit)).toBe(5);
  });

  it("put the nearest to whole first", () => {
    expect(sortKits(kits, { by: "ready", descending: true }).map((kit) => kit.slug)).toEqual([
      "braton_prime_set",
      "zhuge_prime_set",
    ]);
    expect(sortKits(kits, { by: "platinum", descending: true })[0].slug).toBe("zhuge_prime_set");
  });
});
