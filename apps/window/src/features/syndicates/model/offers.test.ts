import type { StandingOffer, Syndicate } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { type OfferFilter, OPEN, shownOffers } from "./offers";

function offer(name: string, change: Partial<StandingOffer>): StandingOffer {
  return {
    item: `/Lotus/${name}`,
    slug: name,
    name,
    image: null,
    vendor: "Отец",
    place: null,
    cost: 5_000,
    rank: null,
    tradable: true,
    platinum: 10,
    ask: 12,
    perDay: 5,
    days: 1,
    owned: 0,
    ...change,
  };
}

const syndicate = {
  tag: "EntratiSyndicate",
  met: true,
  standing: 20_000,
  title: 3,
  leftToday: null,
  offers: [
    offer("receiver", { platinum: 20, perDay: 2 }),
    offer("barrel", { platinum: 10, perDay: 9, owned: 2 }),
    offer("casket", { tradable: false, platinum: null, perDay: null }),
    offer("lens", { cost: 60_000, platinum: 18, rank: 5 }),
  ],
} as Syndicate;

const everything: OfferFilter = { facets: new Map(), platinum: OPEN, volume: OPEN };
const names = (offers: StandingOffer[]) => offers.map((each) => each.name);
const all = () => true;
const none = new Set<string>();
const PLATINUM = { by: "platinum", descending: true } as const;
const CHEAPEST = { by: "cost", descending: false } as const;

describe("a syndicate's offers", () => {
  it("are ordered by platinum, unknown last", () => {
    expect(names(shownOffers(syndicate, everything, PLATINUM, all, none))).toEqual([
      "receiver",
      "lens",
      "barrel",
      "casket",
    ]);
  });

  it("are ordered by standing cost, cheapest first", () => {
    expect(names(shownOffers(syndicate, everything, CHEAPEST, all, none)).at(-1)).toBe("lens");
  });

  it("narrow to what can be sold, opened by rank and afforded", () => {
    const facets = new Map([
      ["tradable", true],
      ["opened", true],
      ["affordable", true],
    ] as const);
    expect(names(shownOffers(syndicate, { ...everything, facets }, PLATINUM, all, none))).toEqual([
      "receiver",
      "barrel",
    ]);
  });

  it("narrow to bounds of platinum and turnover", () => {
    const filter = {
      ...everything,
      platinum: { least: 15, most: null },
      volume: { least: 1, most: 3 },
    };
    expect(names(shownOffers(syndicate, filter, PLATINUM, all, none))).toEqual(["receiver"]);
  });

  it("narrow to what is held and to what is up for sale", () => {
    const held = new Map([["held", true]] as const);
    expect(
      names(shownOffers(syndicate, { ...everything, facets: held }, PLATINUM, all, none)),
    ).toEqual(["barrel"]);
    const listed = new Map([["listed", true]] as const);
    const forSale = new Set(["/Lotus/lens"]);
    expect(
      names(shownOffers(syndicate, { ...everything, facets: listed }, PLATINUM, all, forSale)),
    ).toEqual(["lens"]);
  });
});
