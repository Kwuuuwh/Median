import type { Currency, Good, Offer } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { shelfTabs } from "./shelf";

const good = (item: string, name: string, kind: string): Good => ({
  item,
  name,
  kind,
  image: null,
  known: true,
  slug: null,
  tradable: null,
  vaulted: null,
  prime: false,
  ducats: null,
  makes: null,
  mastered: null,
  collection: null,
});

const offer = (item: string, currency: Currency, amount: number): Offer => ({
  item,
  prices: [{ currency, amount }],
});

const sources = {
  goods: new Map([
    ["/Mod/A", good("/Mod/A", "Мод А", "mod-kind")],
    ["/Mod/B", good("/Mod/B", "Мод Б", "mod-kind")],
    ["/Gun", good("/Gun", "Граката Призма", "rifle")],
    ["/Relic", good("/Relic", "Лит К5", "relic-kind")],
    ["/Skin", good("/Skin", "Скин", "skin")],
  ]),
  kinds: new Map([
    ["mod-kind", "mod"],
    ["rifle", "primary"],
    ["relic-kind", "relic"],
    ["skin", "cosmetic"],
  ]),
  pieces: [{ item: "/Mod/A", variant: "", count: 2, detail: null, equipped: false }],
  quotes: new Map([
    ["/Mod/A", 30],
    ["/Mod/B", 60],
    ["/Gun", 45],
  ]),
  search: "",
  locale: "ru",
};

describe("shelfTabs", () => {
  it("files offers under tabs in their order, paid ones last and unknown packs as bundles", () => {
    const tabs = shelfTabs({
      ...sources,
      offers: [
        offer("/Skin", "ducats", 100),
        offer("/Pack", "aya", 1),
        offer("/Gun", "ducats", 600),
        offer("/Frame", "regalAya", 3),
        offer("/Relic", "aya", 1),
        offer("/Mod/A", "ducats", 300),
      ],
    });
    expect(tabs.map((each) => each.tab)).toEqual([
      "weapons",
      "mods",
      "relics",
      "cosmetics",
      "bundles",
      "regal",
    ]);
  });

  it("puts what goes dearest on the market first", () => {
    const tabs = shelfTabs({
      ...sources,
      offers: [offer("/Mod/A", "ducats", 300), offer("/Mod/B", "ducats", 300)],
    });
    expect(tabs[0].lines.map((line) => line.name)).toEqual(["Мод Б", "Мод А"]);
    expect(tabs[0].lines[0].platinum).toBe(60);
    expect(tabs[0].lines[1].held).toBe(2);
  });

  it("keeps only offers whose name holds every searched word", () => {
    const tabs = shelfTabs({
      ...sources,
      offers: [offer("/Mod/A", "ducats", 1), offer("/Gun", "ducats", 1)],
      search: "призма",
    });
    expect(tabs.flatMap((each) => each.lines.map((line) => line.name))).toEqual(["Граката Призма"]);
  });
});
