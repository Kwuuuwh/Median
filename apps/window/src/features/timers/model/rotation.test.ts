import type { Good, Relic, SetRow } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { rotationOf, wantedParts } from "./rotation";

const good = (item: string, name: string, kind: string, mastered: boolean | null = null): Good => ({
  item,
  name,
  kind,
  image: null,
  known: true,
  slug: null,
  tradable: null,
  vaulted: null,
  prime: true,
  ducats: null,
  makes: null,
  mastered,
  collection: null,
});

const set = (built: string, members: string[]): SetRow => ({
  slug: built,
  name: built,
  ducats: null,
  vaulted: true,
  built,
  members,
  per_set: members.map(() => 1),
});

const relic: Relic = {
  base: "/Relic/K5",
  name: "Реликвия Лит K5",
  image: null,
  era: "lith",
  vaulted: true,
  steps: [],
  holds: [
    {
      item: "/Banshee/Systems",
      name: "Банши Прайм: Система (Чертеж)",
      image: null,
      rarity: "rare",
      count: 1,
      own: 0,
      platinum: 18,
      ducats: 100,
      odds: [],
    },
  ],
};

const sources = {
  offers: [
    { item: "/Braton", prices: [{ currency: "regalAya" as const, amount: 1 }] },
    { item: "/Banshee", prices: [{ currency: "regalAya" as const, amount: 3 }] },
    { item: "/Skin", prices: [{ currency: "regalAya" as const, amount: 1 }] },
  ],
  sets: [set("/Banshee", ["/Banshee/Bp", "/Banshee/Systems"]), set("/Braton", ["/Braton/Bp"])],
  relics: [relic],
  goods: new Map([
    ["/Banshee", good("/Banshee", "Банши Прайм", "warframe")],
    ["/Banshee/Bp", good("/Banshee/Bp", "Банши Прайм (Чертеж)", "blueprint")],
    ["/Braton", good("/Braton", "Братон Прайм", "primary", true)],
  ]),
  held: new Map([["/Banshee/Bp", 1]]),
  quotes: new Map([["/Banshee", 120]]),
  order: (kind: string) => (kind === "warframe" ? 0 : 1),
  locale: "ru",
};

describe("rotationOf", () => {
  it("reads sets on offer with their parts, what drops them and what is held", () => {
    const rotation = rotationOf(sources);
    expect(rotation.map((each) => each.name)).toEqual(["Банши Прайм"]);
    const [banshee] = rotation;
    expect(banshee.platinum).toBe(120);
    expect(banshee.standing.parts).toEqual({ owned: 1, total: 2 });
    expect(banshee.parts.map((part) => part.name)).toEqual(["Чертеж", "Система (Чертеж)"]);
    expect(banshee.parts[1].drops).toEqual([
      { base: "/Relic/K5", name: "Реликвия Лит K5", rarity: "rare" },
    ]);
  });

  it("leaves out a set none of whose parts drop from the offered relics", () => {
    const rotation = rotationOf({ ...sources, relics: [] });
    expect(rotation).toEqual([]);
  });

  it("wants only the missing parts of sets not yet built or mastered", () => {
    expect(wantedParts(rotationOf(sources))).toEqual(new Set(["/Banshee/Systems"]));
  });
});
