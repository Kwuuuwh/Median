import recorded from "virtual:median-snapshot";
import type { CycleCard, Offered } from "@median/ipc";
import "./index";
import { withRivens } from "./rivens";

/** Mock reward screen cards: the first four rewards of the first recorded relic. */
const relic = recorded.getRelics?.[0]?.answer[0];
const state = recorded.getState?.[0]?.answer;
if (relic && state) {
  const kit = relic.holds.slice(0, 4).map((slot, at) => ({
    image: slot.image,
    count: slot.own,
    need: at === 1 ? 2 : 1,
    this: at === 0,
  }));
  const cards = relic.holds.slice(0, 4).map(
    (slot, at): Offered => ({
      favourite: at === 1,
      item: slot.item,
      name: slot.name,
      image: slot.image,
      platinum: slot.platinum,
      ingame: at === 2 ? null : slot.platinum,
      perDayTenths: [3, 127, null, 41][at] ?? null,
      ducats: slot.ducats,
      own: at === 0,
      kit: at === 0 ? kit : [],
      whole: at === 0 ? (slot.platinum ?? 0) * 3 : null,
      reached: at === 0 ? "built" : null,
    }),
  );
  state.game.rewards = { state: "read", cards, fresh: false };
}

/** The riven cycling screen instead, with `?cycling`, `?cycling=one` or `?cycling=reading`. */
const cycling = new URLSearchParams(location.search).get("cycling");
const holdings = recorded.getInventory?.[0]?.answer;
const toxocyst = holdings
  ? withRivens(holdings).pieces.flatMap((piece) =>
      piece.detail?.kind === "riven" && piece.detail.revealed?.weapon.includes("InfVomitGun")
        ? [{ riven: piece.detail.revealed, rank: piece.detail.rank }]
        : [],
    )[0]
  : undefined;
if (state && cycling !== null && toxocyst) {
  const turned = (stat: { tag: string; roll: number }) => ({ ...stat, roll: 2 ** 30 - stat.roll });
  const rolled = {
    ...toxocyst.riven,
    buffs: toxocyst.riven.buffs.map(turned),
    curses: toxocyst.riven.curses.map(turned),
  };
  const cards: CycleCard[] = [
    { side: "chosen", riven: rolled, rank: toxocyst.rank, name: "Croni-satitis", fresh: true },
    {
      side: "left",
      riven: toxocyst.riven,
      rank: toxocyst.rank,
      name: "Hexa-visitis",
      fresh: false,
    },
  ];
  state.game.rewards = { state: "gone" };
  state.game.cycling = {
    state: "open",
    cards: cycling === "reading" ? [] : cycling === "one" ? cards.slice(1) : cards,
    rolls: [{ kuva: 3500, name: cycling === "reading" ? null : "Croni-satitis" }],
    reading: cycling === "reading",
    unread: null,
  };
}
