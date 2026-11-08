import {
  type AppState,
  events,
  type HaulLine,
  type Membership,
  type OwnOrder,
  type Position,
  type Watch,
} from "@median/ipc";
import type { Order } from "@/features/items/model/orders";
import type { Answers } from "./answers";
import { quoteOf, type Snapshot } from "./snapshot";

const TRADERS = [
  "Vexlorn",
  "Ashkara",
  "tenno_mika",
  "Orvisse",
  "Kaelith",
  "Brimstone42",
  "NyxDancer",
  "Solaris_U",
  "Hydroid_Main",
  "Lotusbound",
];
const STATUSES = [
  "ingame",
  "ingame",
  "online",
  "ingame",
  "offline",
  "ingame",
  "online",
  "offline",
  "ingame",
  "online",
];

/** Whether the mock is a released build that finds an update (`?update`). */
const UPDATES = new URLSearchParams(location.search).has("update");
/** How long the mock's update check takes. */
const CHECK_MS = 1_200;
/** Size of the mock's update download, its step and the interval between steps. */
const DOWNLOAD_SIZE = 8_400_000;
const DOWNLOAD_STEP = 600_000;
const DOWNLOAD_TICK_MS = 200;

/** Made-up reputation of the mock's players, assigned to listings in turn. */
const REPUTATION = [412, 3, 87, 1520, 0, 64, 9, 230, 45, 2, 780, 15];

/** Made-up open orders around the recorded price, because the snapshot records no order books. */
function madeUpListings(item: string, base: number, maxRank: number | null, bulk = false): Order[] {
  const name = item.split("/").at(-1) ?? item;
  return TRADERS.map((trader, i): Order => {
    const selling = i < 7;
    const platinum = Math.max(1, selling ? base + i * 2 : base - 2 - i);
    const verb = selling ? "buy" : "sell";
    return {
      name: trader,
      reachable: STATUSES[i] === "ingame",
      status: STATUSES[i],
      selling,
      platinum,
      quantity: 1 + (i % 3),
      perTrade: bulk && selling && i % 3 === 1 ? 6 : 1,
      rank: maxRank === null ? null : i % 2 === 0 ? maxRank : 0,
      subtype: null,
      whisper: `/w ${trader} Hi! I want to ${verb}: "${name}" for ${platinum} platinum.`,
      reputation: REPUTATION[i % REPUTATION.length],
      updated: Math.floor(Date.now() / 1000) - i * 900,
    };
  });
}

/** Kinds warframe.market sells in bulk: relics, arcanes, ayatans, open-world gems and fish. */
const BULK_KINDS = new Set(["relic", "arcane", "ayatan", "gem", "fish"]);

/** Whether an item sells in bulk, guessed from its kind; the snapshot has no market data. */
function bulkOf(snapshot: Snapshot, item: string): boolean {
  const kind = snapshot.getGoods?.[0]?.answer.find((good) => good.item === item)?.kind ?? "";
  return BULK_KINDS.has(kind);
}

/** Days held and cost as a share of the in-game price, for the made-up positions in turn. */
const HELD_FOR = [1, 3, 9, 16, 2, 22];
const PAID = [0.7, 0.85, 1.1, 0.6, 0.95, 1.3];

/** Up to six positions made up from priced holdings, because the snapshot records none. */
function madeUpPositions(snapshot: Snapshot): Position[] {
  const goods = new Map((snapshot.getGoods?.[0]?.answer ?? []).map((good) => [good.item, good]));
  return (snapshot.getPrices?.[0]?.answer ?? [])
    .filter((price) => price.ingame !== null && price.ingame >= 15)
    .slice(0, HELD_FOR.length)
    .map((price, at) => {
      const ingame = price.ingame ?? 0;
      return {
        item: price.item,
        slug: price.slug,
        name: goods.get(price.item)?.name ?? price.slug,
        image: goods.get(price.item)?.image ?? null,
        variant: price.key,
        maxRank: price.maxRank,
        quantity: 1 + (at % 3),
        basis: Math.round(ingame * PAID[at]),
        ingame,
        looked: 600,
        days: HELD_FOR[at],
        asking: at % 2 === 0 ? ingame + 2 : null,
      };
    });
}

/** Delay in milliseconds after a watch is added before the mock triggers it, to show its alarm. */
const RING_AFTER_MS = 3000;

/** Node the mock alarm names for fissure and arrival watches; null for phase watches. */
function ringNode(watch: Watch): string | null {
  if (watch.kind === "fissure") return "SolNode1";
  if (watch.kind === "arrival") return watch.vendor === "baro" ? "SaturnHUB" : "TradeHUB1";
  return null;
}

/** Mock implementations of the state-changing commands; changes are kept in memory. */
export function standActions(snapshot: Snapshot): Answers {
  let favourites = new Set(snapshot.getFavourites?.[0]?.answer ?? []);
  const state = (): AppState | undefined => snapshot.getState?.[0]?.answer;
  const prices = snapshot.getPrices?.[0]?.answer ?? [];
  const goods = new Map(
    (snapshot.getGoods?.[0]?.answer ?? []).map((good) => [good.item, good] as const),
  );
  let haul: HaulLine[] = [
    ...prices
      .filter((price) => (price.worth ?? 0) > 0)
      .slice(0, 14)
      .map((price, at) => {
        const good = goods.get(price.item);
        return {
          item: price.item,
          variant: price.variant,
          count: 1 + (at % 3),
          name: good?.name ?? price.item.split("/").at(-1) ?? price.item,
          image: good?.image ?? null,
          known: true,
          ducats: good?.ducats ?? null,
          worth: price.worth,
        };
      }),
    {
      item: "/Lotus/Types/Recipes/Weapons/WeaponParts/StinoxPrimeSystems",
      variant: "",
      count: 1,
      name: "StinoxPrimeSystems",
      image: null,
      known: false,
      ducats: null,
      worth: null,
    },
  ];
  const haulStarted = Math.floor(Date.now() / 1000);
  const plan = new Set(
    (snapshot.getRoadmap?.[0]?.answer.planned ?? []).map((planned) => planned.item),
  );

  function membership(change: Partial<Membership>): null {
    const current = state();
    if (!current) return null;
    current.membership = { ...current.membership, ...change };
    void events.membershipChanged.emit({ membership: current.membership });
    return null;
  }

  /** Emits `settingsChanged` with the settings `current` holds, as the backend does. */
  function emitSettings(current: AppState): void {
    void events.settingsChanged.emit({
      locale: current.locale,
      accent: current.accent,
      alert: current.alert,
      watches: current.watches,
      updateChannel: current.updateChannel,
    });
  }

  function settings(
    change: Partial<Pick<AppState, "locale" | "accent" | "alert" | "updateChannel">>,
  ): null {
    const current = state();
    if (!current) return null;
    Object.assign(current, change);
    emitSettings(current);
    return null;
  }

  /** Positions taken away from the agent, as `slug|variant`. */
  const released = new Set<string>();

  function ownOrders(): OwnOrder[][] {
    return (snapshot.getOrders ?? []).map((call) => call.answer);
  }

  /** Emits `marketChanged`, as the backend does after any change to the account's orders. */
  function emitMarket(): void {
    void events.marketChanged.emit({ market: { own: Date.now(), failingSince: null } });
  }

  return {
    windowReady: () => undefined,
    setShown: () => undefined,
    syncCatalog: () => null,
    getFavourites: () => [...favourites],
    setFavourite: (item, on) => {
      favourites = new Set(favourites);
      if (on) favourites.add(item);
      else favourites.delete(item);
      return null;
    },
    getListings: (item, variant) => {
      const price = prices.find((each) => each.item === item && each.variant === variant);
      const platinum = price?.ingame ?? quoteOf(snapshot, item)?.platinum;
      const bulk = bulkOf(snapshot, item);
      if (!platinum) return { rows: [], maxRank: null, bulk };
      return {
        rows: madeUpListings(item, platinum, price?.maxRank ?? null, bulk),
        maxRank: price?.maxRank ?? null,
        bulk,
      };
    },
    amendOrder: (id, amend) => {
      let changed: OwnOrder | null = null;
      for (const orders of ownOrders()) {
        const order = orders.find((each) => each.id === id);
        if (!order) continue;
        order.platinum = amend.platinum ?? order.platinum;
        order.quantity = amend.quantity ?? order.quantity;
        order.visible = amend.visible ?? order.visible;
        changed = order;
      }
      if (!changed) throw `the market holds no order ${id}`;
      emitMarket();
      return structuredClone(changed);
    },
    showOrders: (selling, visible) => {
      let touched = 0;
      for (const orders of ownOrders()) {
        touched = 0;
        for (const order of orders) {
          if (order.selling !== selling || order.visible === visible) continue;
          order.visible = visible;
          touched += 1;
        }
      }
      emitMarket();
      return touched;
    },
    refreshOrders: () => {
      emitMarket();
      return null;
    },
    recordExchange: (id, _item, _slug, _variant, _selling, _platinum, quantity) => {
      for (const orders of ownOrders()) {
        const at = orders.findIndex((each) => each.id === id);
        if (at < 0) continue;
        const order = orders[at];
        if (order.quantity > quantity) order.quantity -= quantity;
        else orders.splice(at, 1);
      }
      emitMarket();
      return null;
    },
    handToAgent: () => {
      emitMarket();
      return null;
    },
    handOrderToAgent: (id) => {
      for (const orders of ownOrders()) {
        const order = orders.find((each) => each.id === id);
        if (order) order.byAgent = true;
      }
      emitMarket();
      return null;
    },
    releasePosition: (slug, variant) => {
      released.add(`${slug}|${variant}`);
      emitMarket();
      return null;
    },
    getPositions: () =>
      madeUpPositions(snapshot).filter(
        (position) => !released.has(`${position.slug}|${position.variant}`),
      ),
    withdrawOrders: (ids) => {
      for (const orders of ownOrders()) {
        for (const id of ids) {
          const at = orders.findIndex((order) => order.id === id);
          if (at >= 0) orders.splice(at, 1);
        }
      }
      emitMarket();
      return ids.length;
    },
    placeOrder: (posting) => {
      const price = prices.find((each) => each.item === posting.item);
      const goods = snapshot.getGoods?.[0]?.answer.find((good) => good.item === posting.item);
      for (const orders of ownOrders()) {
        orders.push({
          id: `stand-${Date.now()}`,
          item: posting.item,
          slug: price?.slug ?? "",
          name: goods?.name ?? posting.item,
          image: goods?.image ?? null,
          variant: price?.variant ?? "",
          maxRank: price?.maxRank ?? null,
          selling: posting.selling,
          platinum: posting.platinum,
          quantity: posting.quantity,
          owned: null,
          perTrade: posting.perTrade ?? 1,
          visible: true,
          byAgent: false,
          reference: null,
          basis: null,
          ours: null,
          most: null,
          resale: null,
        });
      }
      emitMarket();
      return null;
    },
    setWatch: (watch, on) => {
      const current = state();
      if (!current) return null;
      const key = JSON.stringify(watch);
      const others = current.watches.filter((each) => JSON.stringify(each) !== key);
      current.watches = on ? [...others, watch] : others;
      emitSettings(current);
      if (on)
        setTimeout(
          () => void events.rang.emit({ alarms: [{ watch, node: ringNode(watch) }] }),
          RING_AFTER_MS,
        );
      return null;
    },
    setAgent: (on) => {
      const current = state();
      if (!current) return null;
      current.agent = { ...current.agent, running: on };
      void events.agentChanged.emit({ agent: current.agent });
      return null;
    },
    setPolicy: (policy) => {
      const current = state();
      if (!current) return null;
      current.policy = policy;
      void events.policyChanged.emit({ policy });
      return null;
    },
    getRoadmap: (lang) => {
      const recorded = snapshot.getRoadmap?.find((call) => call.args[0] === lang)?.answer;
      if (!recorded) throw "no roadmap was recorded";
      const known = new Map(recorded.planned.map((each) => [each.item, each]));
      const mastery = snapshot.getMastery?.find((call) => call.args[0] === lang)?.answer;
      return {
        ...recorded,
        forge: recorded.forge ?? [],
        planned: [...plan]
          .map((item) => ({
            item,
            listed: mastery?.masterable.find((each) => each.item === item),
          }))
          .filter(({ listed }) => listed?.stage !== "mastered")
          .map(
            ({ item, listed }) =>
              known.get(item) ?? {
                item,
                name: listed?.name ?? item,
                image: listed?.image ?? null,
                owned: listed?.owned ?? false,
                remaining: listed?.remaining ?? 0,
              },
          ),
      };
    },
    setRoadmap: (item, on) => {
      if (on) plan.add(item);
      else plan.delete(item);
      return null;
    },
    clearRoadmap: () => {
      plan.clear();
      return null;
    },
    setPartChoice: () => null,
    setPlanned: () => undefined,
    getHaul: () => ({ started: haulStarted, lines: haul }),
    restartHaul: () => {
      haul = [];
      return null;
    },
    getBuild: () =>
      UPDATES
        ? {
            revision: 120,
            hash: "cc465feb9",
            built: Math.floor(Date.now() / 1000) - 3 * 86_400,
            channel: "preview",
            updates: true,
            changes: ["fix(market): read 509 as a connection ceiling, not a rate limit"],
          }
        : {
            revision: 0,
            hash: "",
            built: Math.floor(Date.now() / 1000),
            channel: "dev",
            updates: false,
            changes: [],
          },
    checkUpdate: () =>
      new Promise((found) =>
        setTimeout(
          () =>
            found(
              state()?.updateChannel === "stable"
                ? null
                : {
                    revision: 126,
                    channel: "preview",
                    changes: [
                      "feat(app): sign in to warframe.market and keep the token",
                      "feat(market): carry a token on calls that need one",
                      "feat(account): make the purse part of a reading",
                    ],
                    published: Math.floor(Date.now() / 1000) - 3_600,
                  },
            ),
          CHECK_MS,
        ),
      ),
    installUpdate: () => {
      let downloaded = 0;
      const tick = setInterval(() => {
        downloaded = Math.min(downloaded + DOWNLOAD_STEP, DOWNLOAD_SIZE);
        void events.updateProgressed.emit({ downloaded, total: DOWNLOAD_SIZE });
        if (downloaded === DOWNLOAD_SIZE) clearInterval(tick);
      }, DOWNLOAD_TICK_MS);
      // Never answers.
      return new Promise<null>(() => undefined);
    },
    setUpdateChannel: (updateChannel) => settings({ updateChannel }),
    getCollectorFolder: () => String.raw`C:\Users\tenno\AppData\Local\median\collector`,
    showCollectorFolder: () => null,
    showLogFolder: () => null,
    getGameData: () => ({
      version: state()?.catalog.installed ?? "0.0.0",
      fetched: Math.floor(Date.now() / 1000) - 2 * 86_400,
      gameBuild: "2026.10.28",
    }),
    updateSettled: () => undefined,
    setLocale: (locale) => settings({ locale }),
    setAccent: (accent) => settings({ accent }),
    setAlert: (alert) => settings({ alert }),
    setStatus: (status) => membership({ status }),
    setStatusAuto: (auto) => membership({ auto }),
    signIn: () => membership({ standing: "in", name: "Tenno", status: "invisible" }),
    signOut: () => membership({ standing: "out", name: null }),
  };
}
