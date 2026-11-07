import type { Good, Piece, Priced, SetRow } from "@median/ipc";
import { describe, expect, it } from "vitest";
import {
  freeCount,
  type HeldLine,
  heldLines,
  inPriceRange,
  type LineOrder,
  shortNames,
  sortLines,
  tabOf,
  totals,
} from "./lines";

const good = (item: string, name: string, kind: string, over: Partial<Good> = {}): Good => ({
  item,
  name,
  kind,
  image: null,
  known: true,
  slug: null,
  tradable: true,
  vaulted: null,
  prime: false,
  ducats: null,
  makes: null,
  mastered: null,
  collection: null,
  ...over,
});

const piece = (item: string, count: number, over: Partial<Piece> = {}): Piece => ({
  item,
  variant: "",
  count,
  detail: null,
  equipped: false,
  ...over,
});

const priced = (item: string, ingame: number | null): Priced => ({
  item,
  variant: "",
  ingame,
  looked: null,
  asked: null,
  traded: null,
  maxRank: 10,
  topped: null,
  slug: item.slice(1).toLowerCase(),
  key: "",
  worth: ingame,
});

const set: SetRow = {
  slug: "a_set",
  name: "A: Комплект",
  ducats: null,
  vaulted: true,
  built: "/A",
  members: ["/A1", "/A2"],
  per_set: [1, 1],
};

function lines(pieces: Piece[], goods: Good[], prices: Priced[] = []): HeldLine[] {
  const held = new Map<string, number>();
  for (const each of pieces) held.set(each.item, (held.get(each.item) ?? 0) + each.count);
  return heldLines({
    pieces,
    goods: new Map(goods.map((each) => [each.item, each])),
    classes: new Map([
      ["mod-warframe", "mod"],
      ["component", "component"],
      ["arcane", "arcane"],
    ]),
    sets: [set],
    held,
    price: (item) => prices.find((each) => each.item === item),
    setPrice: (slug) => (slug === "a_set" ? priced("/a_set", 50) : undefined),
  });
}

describe("heldLines", () => {
  it("reads rank, the name and the in-game price", () => {
    const [mod] = lines(
      [piece("/Mod", 2, { detail: { kind: "rank", rank: 3 }, equipped: true })],
      [good("/Mod", "Мод", "mod-warframe")],
      [priced("/Mod", 12)],
    );
    expect(mod).toMatchObject({ name: "Мод", rank: 3, maxRank: 10, platinum: 12 });
    expect(mod?.fitted).toBeNull();
    expect(mod && freeCount(mod)).toBe(1);
  });

  it("adds a line per set held whole and marks parts whose product is held", () => {
    const held = lines(
      [piece("/A1", 2), piece("/A2", 1), piece("/A", 1)],
      [
        good("/A1", "A: Чертёж", "component", { ducats: 15 }),
        good("/A2", "A: Ствол", "component", { ducats: 45 }),
        good("/A", "A", "primary"),
      ],
    );
    const whole = held.find((line) => line.group === "?set");
    expect(whole).toMatchObject({ item: "/A", count: 1, ducats: 60, platinum: 50, vaulted: true });
    expect(held.find((line) => line.item === "/A1")?.built).toBe("crafted");

    const masteredHeld = lines(
      [piece("/A1", 1), piece("/A2", 1), piece("/A", 1)],
      [
        good("/A1", "A: Чертёж", "component", { mastered: true }),
        good("/A2", "A: Ствол", "component", { mastered: null }),
        good("/A", "A", "primary"),
      ],
    );
    expect(masteredHeld.map((line) => line.built)).toEqual([
      "mastered",
      "mastered",
      "mastered",
      "mastered",
    ]);
    const members = new Set(set.members);
    expect(held.map((line) => tabOf(line, members))).toEqual(["parts", "parts", "other", "sets"]);
  });

  it("calls a part spare past one build, and every copy once the set is built", () => {
    const parts = [good("/A1", "A: Чертёж", "component"), good("/A2", "A: Ствол", "component")];
    const loose = lines([piece("/A1", 3), piece("/A2", 1)], parts).filter(
      (line) => line.group !== "?set",
    );
    expect(loose.map((line) => [line.item, line.spare])).toEqual([
      ["/A1", 2],
      ["/A2", 0],
    ]);

    const built = lines([piece("/A1", 1), piece("/A", 1)], [...parts, good("/A", "A", "primary")]);
    expect(built.find((line) => line.item === "/A1")?.spare).toBe(1);
  });

  it("leaves relics and rivens to pages of their own", () => {
    const [relic, riven] = lines(
      [piece("/Relic", 3), piece("/Riven", 1)],
      [good("/Relic", "Лит A1", "relic"), good("/Riven", "Мод разлома", "riven")],
    );
    expect([relic, riven].map((line) => line && tabOf(line, new Set()))).toEqual([null, null]);
  });
});

describe("arcanes", () => {
  const arcane = (rank: number, count: number) =>
    piece("/Arc", count, rank > 0 ? { detail: { kind: "rank", rank } } : {});
  const read = (pieces: Piece[]) =>
    lines(
      pieces,
      [good("/Arc", "Мистификатор", "arcane")],
      [{ ...priced("/Arc", 3), maxRank: 5, topped: 40 }],
    );

  it("counts the rank-zero copies still to fuse, ranked copies fused in", () => {
    expect(read([arcane(3, 1), arcane(0, 4)]).map((line) => line.toMax)).toEqual([7, 7]);
    expect(read([arcane(4, 1), arcane(0, 6)])[0]?.toMax).toBe(0);
  });

  it("calls every copy past the one at the top rank spare", () => {
    const held = read([arcane(5, 2), arcane(2, 1), arcane(0, 3)]);
    expect(held.map((line) => [line.toMax, line.spare])).toEqual([
      [null, 1],
      [null, 1],
      [null, 3],
    ]);
  });

  it("holds an arcane to the price range by what it fetches at the top rank", () => {
    const [line] = read([arcane(0, 1)]);
    expect(line && inPriceRange(line, null, 30)).toBe(false);
    expect(line && inPriceRange(line, null, 40)).toBe(true);
  });
});

describe("shortNames", () => {
  it("drops the words every collection name shares", () => {
    const ru = shortNames([
      "Коллекция мистификаторов: Эйдолон",
      "Коллекция мистификаторов «Спуск»",
      "Коллекция мистификаторов: Неукротимые",
    ]);
    expect([...ru.values()]).toEqual(["Эйдолон", "Спуск", "Неукротимые"]);
    const en = shortNames(["Eidolon Arcane Collection", "Steel Arcane Collection"]);
    expect([...en.values()]).toEqual(["Eidolon", "Steel"]);
  });

  it("keeps a lone name whole", () => {
    const name = "Коллекция мистификаторов: Эйдолон";
    expect(shortNames([name]).get(name)).toBe(name);
  });
});

describe("sortLines", () => {
  const held = lines(
    [piece("/A1", 1), piece("/A2", 5), piece("/C", 2)],
    [
      good("/A1", "А", "component", { ducats: 15 }),
      good("/A2", "Б", "component", { ducats: 45 }),
      good("/C", "В", "component"),
    ],
    [priced("/A1", 30), priced("/A2", 10)],
  ).filter((line) => line.group !== "?set");

  it("puts unknown values last whichever way the sort runs", () => {
    const names = (by: LineOrder, descending: boolean) =>
      sortLines(held, { by, descending }, "ru").map((line) => line.name);
    expect(names("platinum", true)).toEqual(["А", "Б", "В"]);
    expect(names("platinum", false)).toEqual(["Б", "А", "В"]);
    expect(names("ducats", true)).toEqual(["Б", "А", "В"]);
    expect(names("count", true)).toEqual(["Б", "В", "А"]);
    expect(names("name", true)).toEqual(["В", "Б", "А"]);
  });

  it("adds up what every copy is expected to fetch, and its ducats", () => {
    expect(totals(held)).toEqual({ platinum: 80, ducats: 240 });
  });
});

describe("the price range", () => {
  const priced = (platinum: number | null) => ({ platinum }) as HeldLine;

  it("holds what falls between the bounds, a bound left empty holding anything", () => {
    expect(inPriceRange(priced(15), 10, 20)).toBe(true);
    expect(inPriceRange(priced(25), 10, 20)).toBe(false);
    expect(inPriceRange(priced(25), 10, null)).toBe(true);
    expect(inPriceRange(priced(5), null, null)).toBe(true);
  });

  it("leaves out an unpriced line once any bound is set", () => {
    expect(inPriceRange(priced(null), 1, null)).toBe(false);
    expect(inPriceRange(priced(null), null, null)).toBe(true);
  });
});
