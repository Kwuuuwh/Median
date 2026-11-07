import {
  type Card,
  type Collectible,
  type Curve,
  events,
  type Good,
  type Holdings,
  type Quote,
  type Relic,
  type Reward,
} from "@median/ipc";
import { DAY } from "@/shared/lib/time";
import type { Answer, Answers, Command } from "./answers";
import {
  appraise,
  ceiling,
  choices,
  demand,
  madeUpAuctions,
  madeUpBuys,
  madeUpPicks,
  rivenName,
  rivenWeapons,
  rolls,
  withRivens,
} from "./rivens";

/** One call recorded by the snapshot binary: its arguments and response. */
type Call<K extends Command> = { args: Parameters<Answers[K] & {}>; answer: Answer<K> };

/** Recorded calls, by command. */
export type Snapshot = { [K in Command]?: Call<K>[] };

/** Recorded response to exactly these arguments; throws when none was recorded. */
function replay<K extends Command>(snapshot: Snapshot, command: K, args: unknown[]): Answer<K> {
  const calls = (snapshot[command] ?? []) as Call<K>[];
  const wanted = JSON.stringify(args);
  const call = calls.find((recorded) => JSON.stringify(recorded.args) === wanted);
  if (!call) throw `${command}(${wanted}) was not recorded`;
  return structuredClone(call.answer);
}

/** Recorded catalog rows of one language, by item key. */
export function goodsIn(snapshot: Snapshot, lang: string): Map<string, Good> {
  const call = snapshot.getGoods?.find((recorded) => recorded.args[0] === lang);
  return new Map((call?.answer ?? []).map((good) => [good.item, good]));
}

/**
 * Made-up market history of `days` days around `platinum`, the same for every call with the same
 * item: closed trades are higher until two days ago, plus sell and buy order series.
 */
function inventedCurve(item: string, platinum: number, days: number, topRank: boolean): Curve {
  let seed = 0;
  for (const char of item) seed = (seed * 31 + char.charCodeAt(0)) >>> 0;
  const random = () => {
    seed = (seed * 1_103_515_245 + 12_345) >>> 0;
    return seed / 2 ** 32;
  };
  const maxRank = item.includes("/Mods/") ? 10 : null;
  const level = maxRank !== null && topRank ? platinum * 4 : platinum;
  const dearer = 1.1 + random() * 0.6;
  const today = Math.floor(Date.now() / 1000 / DAY) * DAY;
  const at = (index: number) => today - (days - 1 - index) * DAY;
  const near = (price: number) => Math.max(1, Math.round(price * (0.92 + random() * 0.16)));
  const standing = (share: number) =>
    Array.from({ length: days }, (_, index) => ({
      at: at(index),
      best: near(level * share),
      median: near(level * share * (share > 1 ? 1.2 : 0.8)),
      orders: 5 + Math.floor(random() * 40),
    }));
  return {
    closed: Array.from({ length: days }, (_, index) => {
      const ago = days - 1 - index;
      return {
        at: at(index),
        platinum: near(ago < 2 ? level : level * dearer),
        weight: 1 + Math.floor(random() * 12),
      };
    }),
    selling: standing(1.15),
    buying: standing(0.8),
    maxRank,
  };
}

/**
 * Recorded quote of an item; otherwise one made up from held prices, relic reward prices or, for a
 * relic, its intact run value; null when none applies.
 */
export function quoteOf(snapshot: Snapshot, item: string): Quote | null {
  const recorded = snapshot.getQuotes?.[0]?.answer.find((quote) => quote.item === item);
  if (recorded?.platinum) return recorded;
  const held = snapshot.getPrices?.[0]?.answer.find((price) => price.item === item && price.ingame);
  if (held?.ingame)
    return { item, platinum: held.ingame, pair: { slug: held.slug, key: held.key } };
  const slot = [
    ...(snapshot.getRelicsOf?.[0]?.answer ?? []),
    ...(snapshot.getRelics?.[0]?.answer ?? []),
  ]
    .flatMap((relic) => relic.holds)
    .find((each) => each.item === item && each.platinum);
  if (slot?.platinum) return { item, platinum: slot.platinum, pair: null };
  const relic = snapshot.getRelicsOf?.[0]?.answer.find((each) => each.steps[0]?.item === item);
  const intact = relic?.steps[0];
  return intact
    ? { item, platinum: Math.max(1, Math.round(intact.platinum / 14)), pair: null }
    : null;
}

/** Every recorded relic of one language: held relics and relics of the vendor offers. */
function relicsIn(snapshot: Snapshot, lang: string): Relic[] {
  const byBase = new Map<string, Relic>();
  for (const call of [...(snapshot.getRelics ?? []), ...(snapshot.getRelicsOf ?? [])]) {
    if (call.args[0] !== lang) continue;
    for (const relic of call.answer) byBase.set(relic.base, relic);
  }
  return [...byBase.values()];
}

/** Item card from the snapshot's catalog rows, sets and relics; no drops, vendors or research. */
function cardOf(snapshot: Snapshot, lang: string, item: string): Card {
  const goods = goodsIn(snapshot, lang);
  const good = goods.get(item) ?? {
    item,
    name: item.split("/").at(-1) ?? item,
    kind: "",
    image: null,
    known: false,
    slug: null,
    tradable: null,
    vaulted: null,
    prime: false,
    ducats: null,
    makes: null,
    mastered: null,
    collection: null,
  };
  const relics = relicsIn(snapshot, lang);
  const found: Reward[] = relics.flatMap((relic) => {
    const slot = relic.holds.find((each) => each.item === item);
    if (!slot) return [];
    return relic.steps.map((step, index) => ({
      item: step.item,
      name: relic.name,
      image: relic.image,
      rarity: slot.rarity,
      refinement: step.refinement,
      chance: slot.odds[index]?.chance ?? 0,
      count: slot.count,
    }));
  });
  const opened = relics.find((relic) => relic.steps.some((step) => step.item === item));
  const at = opened?.steps.findIndex((step) => step.item === item) ?? 0;
  const inside: Reward[] = (opened?.holds ?? []).map((slot) => ({
    item: slot.item,
    name: slot.name,
    image: slot.image,
    rarity: slot.rarity,
    refinement: null,
    chance: slot.odds[at]?.chance ?? 0,
    count: slot.count,
  }));
  const set = snapshot.getSets
    ?.find((call) => call.args[0] === lang)
    ?.answer.find((each) => each.built === item);
  const recipes = set
    ? [
        {
          blueprint: set.members.find((member) => member.endsWith("Blueprint")) ?? set.members[0],
          parts: set.members
            .filter((member) => !member.endsWith("Blueprint"))
            .map((member) => ({
              item: member,
              name: goods.get(member)?.name ?? member,
              image: goods.get(member)?.image ?? null,
              count: 1,
              class: "component",
            })),
          credits: null,
          time: null,
          rush: null,
        },
      ]
    : [];
  return {
    good,
    sold: [],
    inside,
    found,
    drops: [],
    research: [],
    recipes,
    neededBy: [],
    way: null,
  };
}

/** Classes of items that other items can be installed on. */
const CARRIERS = new Set([
  "warframe",
  "primary",
  "secondary",
  "melee",
  "companion",
  "archwing-weapon",
]);

/**
 * Holdings with made-up `fitted` items for every piece marked equipped, because the backend reports
 * only that a piece is equipped.
 */
function withFittings(snapshot: Snapshot): Holdings | undefined {
  const holdings = snapshot.getInventory?.[0]?.answer;
  if (!holdings) return undefined;
  const goods = goodsIn(snapshot, "en");
  const classes = new Map(
    (snapshot.getTaxonomy?.[0]?.answer.kinds ?? []).map((kind) => [kind.slug, kind.class]),
  );
  const carriers = holdings.pieces
    .map((piece) => piece.item)
    .filter((item) => CARRIERS.has(classes.get(goods.get(item)?.kind ?? "") ?? ""));
  const pieces = holdings.pieces.map((piece) => {
    if (!piece.equipped || carriers.length === 0) return piece;
    let seed = 0;
    for (const char of piece.item) seed = (seed * 31 + char.charCodeAt(0)) >>> 0;
    const count = 1 + (seed % 9);
    const fitted = Array.from(
      new Set(
        Array.from({ length: count }, (_, at) => carriers[(seed + at * 7) % carriers.length]),
      ),
    );
    return { ...piece, fitted };
  });
  return { ...holdings, pieces };
}

/** Classes the catalog page lists. */
const COLLECTED: ReadonlySet<string> = new Set([
  "warframe",
  "primary",
  "secondary",
  "melee",
  "archwing-weapon",
  "companion",
  "companion-weapon",
  "mod",
  "arcane",
]);

/** Classes whose items give mastery. */
const MASTERED_CLASSES: ReadonlySet<string> = new Set([
  "warframe",
  "primary",
  "secondary",
  "melee",
  "archwing-weapon",
  "companion",
  "companion-weapon",
]);

/** Mastery rank of the mock account. */
const MOCK_RANK = 14;

/** Made-up catalog page rows from the recorded catalog and holdings. */
function madeUpCatalogue(snapshot: Snapshot, lang: string, holdings?: Holdings): Collectible[] {
  const classes = new Map(
    (snapshot.getTaxonomy?.[0]?.answer.kinds ?? []).map((kind) => [kind.slug, kind.class]),
  );
  const owned = new Map<string, number>();
  for (const piece of holdings?.pieces ?? []) {
    owned.set(piece.item, (owned.get(piece.item) ?? 0) + piece.count);
  }
  return [...goodsIn(snapshot, lang).values()].flatMap((good) => {
    const className = classes.get(good.kind) ?? "";
    if (!COLLECTED.has(className)) return [];
    let seed = 0;
    for (const char of good.item) seed = (seed * 31 + char.charCodeAt(0)) >>> 0;
    const gives = MASTERED_CLASSES.has(className);
    const held = owned.get(good.item) ?? 0;
    const required = gives ? seed % 18 : 0;
    return [
      {
        item: good.item,
        name: good.name,
        image: good.image,
        class: className,
        kind: good.kind,
        prime: good.prime,
        vaulted: good.vaulted ?? false,
        tradable: good.tradable ?? false,
        owned: held,
        mastered: gives ? held > 0 && seed % 3 !== 0 : null,
        ready: gives ? held === 0 && seed % 7 === 0 : null,
        required,
        reachable: required <= MOCK_RANK,
        relics: good.prime && seed % 4 === 0,
      },
    ];
  });
}

/**
 * Mock commands from the snapshot: exact replay of recorded calls, except commands that select
 * from a larger recorded response, plus made-up riven data.
 */
export function snapshotAnswers(snapshot: Snapshot): Answers {
  const answers: Record<string, (...args: unknown[]) => unknown> = {};
  for (const command of Object.keys(snapshot) as Command[]) {
    answers[command] = (...args) => replay(snapshot, command, args);
  }
  const fitted = withFittings(snapshot);
  const holdings = fitted && withRivens(fitted);
  let auctions = holdings ? madeUpAuctions(holdings) : [];
  let buys = holdings ? madeUpBuys(holdings) : [];
  const state = snapshot.getState?.[0]?.answer;
  if (state) state.agent = { ...state.agent, picks: holdings ? madeUpPicks(holdings) : [] };
  const dropPick = (id: string) => {
    if (!state) return;
    state.agent = { ...state.agent, picks: state.agent.picks.filter((pick) => pick.id !== id) };
    void events.agentChanged.emit({ agent: state.agent });
  };
  return {
    ...(answers as Answers),
    ...(holdings ? { getInventory: () => structuredClone(holdings) } : {}),
    getGoods: (lang, items) => {
      const known = goodsIn(snapshot, lang);
      const unknown = items.filter((item) => !known.has(item));
      return [
        ...items.flatMap((item) => {
          const good = known.get(item);
          return good ? [structuredClone(good)] : [];
        }),
        ...rivenWeapons(lang, unknown),
      ];
    },
    getCatalogue: (lang) => madeUpCatalogue(snapshot, lang, holdings),
    getRivenAppraisal: (lang, riven, rank) => appraise(lang, riven, rank),
    getRivenNames: (rivens) => rivens.map(rivenName),
    getRivenChoices: (lang, riven) => choices(lang, riven),
    getRivenBand: (riven) => choices("en", riven)?.sell ?? null,
    getRivenCeiling: (weapon) => ceiling(weapon),
    getRivenDemand: (weapon) => demand(weapon),
    openAuction: () => null,
    getRivenRolls: (riven) => rolls(riven),
    getOwnAuctions: () => structuredClone(auctions),
    getRivenBuys: () => structuredClone(buys),
    recordRivenBuy: (id, price) => {
      const pick = state?.agent.picks.find((each) => each.id === id);
      if (!pick) throw `no pick ${id}`;
      buys = [
        {
          auction: pick.id,
          name: pick.name,
          riven: pick.riven,
          rank: pick.rank,
          price,
          bought: Math.floor(Date.now() / 1000),
          sold: null,
          sale: null,
        },
        ...buys,
      ];
      dropPick(id);
      return null;
    },
    dismissRivenPick: (id) => {
      dropPick(id);
      return null;
    },
    recordRivenSale: (auction, price) => {
      buys = buys.map((buy) =>
        buy.auction === auction
          ? { ...buy, sold: Math.floor(Date.now() / 1000), sale: price }
          : buy,
      );
      return null;
    },
    putUpRiven: (riven, rank, starting, buyout) => {
      const id = `stand-${Date.now()}`;
      auctions.unshift({
        id,
        weapon: riven.weapon,
        riven,
        name: (rivenName(riven) ?? "").toLowerCase(),
        rank,
        startingPrice: starting,
        buyoutPrice: buyout,
        topBid: null,
        isDirectSell: buyout === starting,
        visible: true,
        created: new Date().toISOString(),
      });
      return id;
    },
    changeRivenAuction: (id, starting, buyout, visible) => {
      const auction = auctions.find((each) => each.id === id);
      if (!auction) throw `the market holds no auction ${id}`;
      auction.startingPrice = starting;
      auction.buyoutPrice = buyout ?? auction.buyoutPrice;
      auction.visible = visible;
      return null;
    },
    closeRivenAuction: (id) => {
      auctions = auctions.filter((each) => each.id !== id);
      return null;
    },
    getNodes: (nodeKeys) => {
      const known = new Map(
        (snapshot.getNodes?.[0]?.answer ?? []).map((node) => [node.node, node]),
      );
      return nodeKeys.flatMap((key) => {
        const node = known.get(key);
        return node ? [structuredClone(node)] : [];
      });
    },
    getRelicsOf: (lang, items) => {
      const asked = new Set(items);
      const call = snapshot.getRelicsOf?.find((recorded) => recorded.args[0] === lang);
      return structuredClone(
        (call?.answer ?? []).filter((relic) => relic.steps.some((step) => asked.has(step.item))),
      );
    },
    getQuotes: (items) =>
      structuredClone(
        items.flatMap((item) => {
          const quote = quoteOf(snapshot, item);
          return quote ? [quote] : [];
        }),
      ),
    getCurve: (item, variant, days, topRank) => {
      const recorded = snapshot.getCurve?.find(
        (call) =>
          call.args[0] === item &&
          call.args[1] === variant &&
          call.args[2] === days &&
          (call.args[3] ?? false) === topRank,
      );
      if (recorded) {
        return structuredClone({
          ...recorded.answer,
          selling: recorded.answer.selling ?? [],
          buying: recorded.answer.buying ?? [],
          maxRank: recorded.answer.maxRank ?? null,
        });
      }
      const platinum = quoteOf(snapshot, item)?.platinum;
      return platinum
        ? inventedCurve(item, platinum, days, topRank)
        : { closed: [], selling: [], buying: [], maxRank: null };
    },
    getCard: (lang, item) => {
      const recorded = snapshot.getCard?.find(
        (call) => call.args[0] === lang && call.args[1] === item,
      );
      return structuredClone(recorded ? recorded.answer : cardOf(snapshot, lang, item));
    },
    getOtherName: (lang, item) =>
      goodsIn(snapshot, lang === "ru" ? "en" : "ru").get(item)?.name ?? null,
  };
}
