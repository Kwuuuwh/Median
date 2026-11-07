import type {
  AppraisedStat,
  Good,
  Holdings,
  OwnAuction,
  Piece,
  Polarity,
  RerollOdds,
  Riven,
  RivenAppraisal,
  RivenBuy,
  RivenChoices,
  RivenDemand,
  RivenGrade,
  RivenPick,
  StatVerdict,
} from "@median/ipc";
import { DAY } from "@/shared/lib/time";

type Weapon = { en: string; ru: string; class: string; disposition: number; image: string };

/** Weapons of the rivens in the recorded snapshot, as the catalog names them. */
const WEAPONS: Record<string, Weapon> = {
  "/Lotus/Weapons/ClanTech/Chemical/FlameThrower": {
    en: "Ignis",
    ru: "Игнис",
    class: "rifle",
    image: "2bc3cecbab759d60",
    disposition: 0.7,
  },
  "/Lotus/Weapons/ClanTech/Chemical/RocketLauncher": {
    en: "Ogris",
    ru: "Огрис",
    class: "rifle",
    image: "21db43f9693b35de",
    disposition: 1.3,
  },
  "/Lotus/Weapons/ClanTech/Energy/CrpLaserRifle": {
    en: "Flux Rifle",
    ru: "Потоковая Винтовка",
    class: "rifle",
    image: "ee7522b1251857b1",
    disposition: 1.55,
  },
  "/Lotus/Weapons/Corpus/Bow/Longbow/CrpBow": {
    en: "Lenz",
    ru: "Ленц",
    class: "rifle",
    image: "b27102b182deae1c",
    disposition: 1.05,
  },
  "/Lotus/Weapons/Grineer/LongGuns/GrineerAssaultRifle/GrnAssaultRifle": {
    en: "Grakata",
    ru: "Граката",
    class: "rifle",
    image: "e62185a6c9ab350f",
    disposition: 1.35,
  },
  "/Lotus/Weapons/Grineer/LongGuns/GrnHarpoonGun/GrnHarpoonGun": {
    en: "Harpak",
    ru: "Харпак",
    class: "rifle",
    image: "33b5a2a36e8d318e",
    disposition: 1.55,
  },
  "/Lotus/Weapons/Infested/Pistols/InfVomitGun/InfVomitGunWep": {
    en: "Dual Toxocyst",
    ru: "Парные Токсоцисты",
    class: "pistol",
    image: "1f065dcf142f9052",
    disposition: 1.35,
  },
  "/Lotus/Weapons/Tenno/Archwing/Primary/LaunchGrenade/ArchCannon": {
    en: "Corvas",
    ru: "Корвас",
    class: "archgun",
    image: "a27883decc9d2601",
    disposition: 1.2,
  },
  "/Lotus/Weapons/Tenno/Archwing/Primary/RocketArtillery/ArchRocketCrossbow": {
    en: "Fluctus",
    ru: "Флактус",
    class: "archgun",
    image: "206c0d7171bf6ae2",
    disposition: 1.1,
  },
  "/Lotus/Weapons/Tenno/Melee/Swords/TennoSai/TennoSais": {
    en: "Okina",
    ru: "Окина",
    class: "melee",
    image: "f55ad695f80efcf8",
    disposition: 1.4,
  },
  "/Lotus/Weapons/Tenno/Pistols/SapientPistol/SapientPistol": {
    en: "Akarius",
    ru: "Акариус",
    class: "pistol",
    image: "7112dd9b6c4ee559",
    disposition: 1.05,
  },
};

type Shown = { base: number; form: AppraisedStat["form"]; en: string; ru: string | null };

/** Rifle-pool stats of the snapshot's rivens: base value, form and the label after the number. */
const STATS: Record<string, Shown> = {
  WeaponAmmoMaxMod: {
    base: 0.00555,
    form: "percent",
    en: "% Ammo Maximum",
    ru: "% максимум патронов",
  },
  WeaponClipMaxMod: {
    base: 0.005555,
    form: "percent",
    en: "% Magazine Capacity",
    ru: "% к вместимости магазина",
  },
  WeaponCritChanceMod: { base: 0.016666, form: "percent", en: "% Critical Chance", ru: null },
  WeaponCritDamageMod: {
    base: 0.013333,
    form: "percent",
    en: "% Critical Damage",
    ru: "% крит. урона",
  },
  WeaponDamageAmountMod: { base: 0.0183333, form: "percent", en: "% Damage", ru: "% урона" },
  WeaponFactionDamageCorpus: {
    base: 0.005,
    form: "multiplier",
    en: "Damage to Corpus",
    ru: "урона Корпусу",
  },
  WeaponFactionDamageGrineer: {
    base: 0.005,
    form: "multiplier",
    en: "Damage to Grineer",
    ru: "урона Гринир",
  },
  WeaponFactionDamageInfested: {
    base: 0.005,
    form: "multiplier",
    en: "Damage to Infested",
    ru: "урона Заражённым",
  },
  WeaponFireDamageMod: { base: 0.01, form: "percent", en: "% Heat", ru: "% урона огнем" },
  WeaponFireIterationsMod: {
    base: 0.01,
    form: "percent",
    en: "% Multishot",
    ru: "% мультивыстрел",
  },
  WeaponFireRateMod: {
    base: 0.00667,
    form: "percent",
    en: "% Fire Rate (x2 for Bows)",
    ru: "% к скорострельности (x2 для луков)",
  },
  WeaponFreezeDamageMod: { base: 0.01, form: "percent", en: "% Cold", ru: "% урона холодом" },
  WeaponImpactDamageMod: {
    base: 0.013333,
    form: "percent",
    en: "% Impact",
    ru: "% ударного урона",
  },
  WeaponProcTimeMod: {
    base: 0.01111,
    form: "percent",
    en: "% Status Duration",
    ru: "% к длительности статуса",
  },
  WeaponPunctureDepthMod: {
    base: 0.03,
    form: "plain",
    en: "Punch Through",
    ru: "к пронзанию навылет",
  },
  WeaponRecoilReductionMod: {
    base: -0.01,
    form: "percent",
    en: "% Weapon Recoil",
    ru: "% к отдаче",
  },
  WeaponSlashDamageMod: {
    base: 0.013333,
    form: "percent",
    en: "% Slash",
    ru: "% разрезающего урона",
  },
  WeaponStunChanceMod: {
    base: 0.01,
    form: "percent",
    en: "% Status Chance",
    ru: "% к шансу статуса",
  },
  WeaponZoomFovMod: { base: 0.006666, form: "percent", en: "% Zoom", ru: "% приближение" },
};

/** Riven name syllables of each stat, as prefix and suffix, as stored in the catalog. */
const AFFIXES: Record<string, [string, string]> = {
  WeaponAmmoMaxMod: ["ampi", "bin"],
  WeaponClipMaxMod: ["arma", "tin"],
  WeaponCritChanceMod: ["crita", "cron"],
  WeaponCritDamageMod: ["acri", "tis"],
  WeaponDamageAmountMod: ["visi", "ata"],
  WeaponFactionDamageCorpus: ["manti", "tron"],
  WeaponFactionDamageGrineer: ["argi", "con"],
  WeaponFactionDamageInfested: ["pura", "ada"],
  WeaponFireDamageMod: ["igni", "pha"],
  WeaponFireIterationsMod: ["sati", "can"],
  WeaponFireRateMod: ["croni", "dra"],
  WeaponFreezeDamageMod: ["geli", "do"],
  WeaponImpactDamageMod: ["magna", "ton"],
  WeaponProcTimeMod: ["deci", "des"],
  WeaponPunctureDepthMod: ["lexi", "nok"],
  WeaponRecoilReductionMod: ["zeti", "mag"],
  WeaponSlashDamageMod: ["sci", "sus"],
  WeaponStunChanceMod: ["hexa", "dex"],
  WeaponZoomFovMod: ["hera", "lis"],
};

/**
 * In-game riven name, built as the app builds it: prefixes of the higher-rolled buffs joined by
 * `-`, then the suffix of the lowest-rolled one; null for a stat without syllables.
 */
export function rivenName(riven: Riven): string | null {
  const buffs = [...riven.buffs].sort((one, other) => other.roll - one.roll);
  const last = buffs.pop();
  if (!last) return null;
  const prefixes = buffs.map((stat) => AFFIXES[stat.tag]?.[0]);
  const suffix = AFFIXES[last.tag]?.[1];
  if (!suffix || prefixes.some((prefix) => !prefix)) return null;
  const name = `${prefixes.join("-")}${suffix}`;
  return name.charAt(0).toUpperCase() + name.slice(1);
}

const WANTED_BUFFS = new Set([
  "WeaponDamageAmountMod",
  "WeaponCritChanceMod",
  "WeaponCritDamageMod",
  "WeaponFireIterationsMod",
]);
const FINE_BUFFS = new Set([
  "WeaponFireRateMod",
  "WeaponStunChanceMod",
  "WeaponFireDamageMod",
  "WeaponFreezeDamageMod",
]);
const WANTED_CURSES = new Set([
  "WeaponZoomFovMod",
  "WeaponRecoilReductionMod",
  "WeaponImpactDamageMod",
  "WeaponAmmoMaxMod",
  "WeaponProcTimeMod",
]);
const FINE_CURSES = new Set(["WeaponSlashDamageMod", "WeaponClipMaxMod"]);

const VALUED = new Set(["rifle", "shotgun", "pistol"]);

const POLARITIES: Record<string, Polarity> = {
  AP_ATTACK: "madurai",
  AP_DEFENSE: "vazarin",
  AP_TACTIC: "naramon",
};

type Fingeprint = {
  compat?: string;
  lvl?: number;
  lvlReq?: number;
  rerolls?: number;
  pol?: string;
  buffs?: { Tag: string; Value: number }[];
  curses?: { Tag: string; Value: number }[];
};

/** A riven piece with its detail parsed from the variant fingerprint, as the app parses it. */
function revealed(piece: Piece): Piece {
  if (!piece.item.includes("/Randomized/") || !piece.variant.startsWith("{")) return piece;
  const read = JSON.parse(piece.variant) as Fingeprint;
  if (!read.compat) return piece;
  const stat = (each: { Tag: string; Value: number }) => ({ tag: each.Tag, roll: each.Value });
  return {
    ...piece,
    detail: {
      kind: "riven",
      rank: read.lvl ?? 0,
      revealed: {
        weapon: read.compat,
        mastery: read.lvlReq ?? 0,
        rerolls: read.rerolls ?? 0,
        polarity: read.pol ? (POLARITIES[read.pol] ?? null) : null,
        buffs: (read.buffs ?? []).map(stat),
        curses: (read.curses ?? []).map(stat),
      },
    },
  };
}

/** Holdings with the detail of every riven piece parsed. */
export function withRivens(holdings: Holdings): Holdings {
  return { ...holdings, pieces: holdings.pieces.map(revealed) };
}

/** Catalog rows of the snapshot's riven weapons, for items the recorded catalog rows lack. */
export function rivenWeapons(lang: string, items: readonly string[]): Good[] {
  return items.flatMap((item) => {
    const weapon = WEAPONS[item];
    if (!weapon) return [];
    return [
      {
        item,
        name: lang === "ru" ? weapon.ru : weapon.en,
        kind: "primary",
        image: weapon.image,
        known: true,
        slug: null,
        tradable: false,
        vaulted: null,
        prime: false,
        ducats: null,
        makes: null,
        mastered: null,
        collection: null,
      },
    ];
  });
}

const SPLITS: Record<string, [number, number | null]> = {
  "2+0": [0.99, null],
  "2+1": [1.2375, -0.495],
  "3+0": [0.75, null],
  "3+1": [0.9375, -0.75],
};

/**
 * Made-up appraisal in the app's response shape: stat values as the game would show them and
 * verdicts by a fixed rule; null for a weapon class the mock does not appraise.
 */
export function appraise(lang: string, riven: Riven, rank: number): RivenAppraisal | null {
  const weapon = WEAPONS[riven.weapon];
  if (!weapon || !VALUED.has(weapon.class)) return null;
  const [buffFactor, curseFactor] = SPLITS[`${riven.buffs.length}+${riven.curses.length}`] ?? [
    1, -1,
  ];
  const ranked = (Math.min(rank, 8) + 1) / 9;
  const side = (isCurse: boolean) => (stat: { tag: string; roll: number }) => {
    const shown = STATS[stat.tag] ?? {
      base: 0.01,
      form: "percent" as const,
      en: stat.tag,
      ru: null,
    };
    const factor = isCurse ? (curseFactor ?? -0.75) : buffFactor;
    const quality = 0.9 + (0.2 * stat.roll) / 2 ** 30;
    const verdict: StatVerdict = isCurse
      ? WANTED_CURSES.has(stat.tag) || FINE_CURSES.has(stat.tag)
        ? "harmless"
        : WANTED_BUFFS.has(stat.tag)
          ? "harmful"
          : "useless"
      : WANTED_BUFFS.has(stat.tag)
        ? "decisive"
        : FINE_BUFFS.has(stat.tag)
          ? "wanted"
          : "useless";
    return {
      tag: stat.tag,
      isCurse,
      label: (lang === "ru" ? shown.ru : null) ?? shown.en,
      form: shown.form,
      amount: shown.base * 90 * weapon.disposition * factor * quality * ranked,
      rollShare: stat.roll / 2 ** 30,
      verdict,
    } satisfies AppraisedStat;
  };
  const stats = [...riven.buffs.map(side(false)), ...riven.curses.map(side(true))];
  const helping = stats.filter(
    (stat) => !stat.isCurse && (stat.verdict === "decisive" || stat.verdict === "wanted"),
  ).length;
  const hurts = stats.some((stat) => stat.verdict === "harmful");
  const grade: RivenGrade = hurts
    ? helping >= 3
      ? "potential"
      : "bad"
    : helping >= 3
      ? "perfect"
      : helping === 2
        ? "good"
        : helping === 1
          ? "potential"
          : "bad";
  return { grade, stats };
}

const GRADE_PRICE: Record<RivenGrade, number> = {
  bad: 60,
  potential: 180,
  good: 420,
  perfect: 900,
};

/** Made-up price of similar rivens: a fixed price per grade plus the reroll count. */
function middleOf(appraisal: RivenAppraisal, riven: Riven): number {
  return GRADE_PRICE[appraisal.grade ?? "bad"] + riven.rerolls;
}

/** Made-up highest price of a rolled riven of a weapon: 1500 / disposition, rounded. */
export function ceiling(weapon: string): number | null {
  const known = WEAPONS[weapon];
  return known && VALUED.has(known.class) ? Math.round(1500 / known.disposition) : null;
}

/** Made-up demand for a weapon's rivens, chosen by the length of the weapon's English name. */
export function demand(weapon: string): RivenDemand | null {
  const known = WEAPONS[weapon];
  if (!known || !VALUED.has(known.class)) return null;
  const levels: RivenDemand[] = ["lively", "slow", "dead", "unclear"];
  return levels[known.en.length % levels.length] ?? null;
}

/** Made-up per-lock odds of a higher-priced roll and its average price; the last lock is ~0. */
export function rolls(riven: Riven): RerollOdds[] | null {
  const appraisal = appraise("en", riven, 8);
  if (!appraisal) return null;
  const middle = middleOf(appraisal, riven);
  const locks = [null, ...riven.buffs.map((stat) => stat.tag)];
  return locks.map((locked, at) => {
    const chance = at === locks.length - 1 ? 0.0004 : 0.42 - at * 0.09;
    return {
      locked,
      chance,
      expectedKuva: 1500 / chance,
      betterMean: middle * (1.4 + at * 0.1),
    };
  });
}

/** Made-up market data for a riven: a price band around its grade price and 7 similar auctions. */
export function choices(lang: string, riven: Riven): RivenChoices | null {
  const appraisal = appraise(lang, riven, 8);
  if (!appraisal) return null;
  const middle = middleOf(appraisal, riven);
  return {
    sell: {
      low: Math.round(middle * 0.8),
      middle,
      high: Math.round(middle * 1.35),
      lots: 40 + riven.rerolls,
      basis: riven.buffs.length === 3 ? "lots" : "fewLots",
      isCapped: riven.rerolls > 50,
    },
    transmute: 3.5,
    analogs: Array.from({ length: 7 }, (_, at) => ({
      id: `mock-${at}`,
      price: Math.round(middle * (0.7 + at * 0.12)),
      rank: at % 3 === 0 ? 0 : 8,
      rerolls: at * 7,
      stats: appraisal.stats.map((stat, place) => ({
        tag: at % 2 === 1 && place === 1 ? "WeaponFireDamageMod" : stat.tag,
        label:
          at % 2 === 1 && place === 1 ? (lang === "ru" ? "% урона огнем" : "% Heat") : stat.label,
        form: stat.form,
        amount: (stat.amount ?? 0) * (0.92 + ((at + place) % 4) * 0.05),
        isCurse: stat.isCurse,
      })),
      seller:
        ["Tenno_Rex", "kuva_queen", "Lotus42", "VoidWalker", "rivenmaster", "Ordis", "glassmaker"][
          at
        ] ?? null,
      status: ["ingame", "online", "offline"][at % 3] ?? null,
      likeness: at < 4 ? 1 : 0.75,
    })),
  };
}

/** Made-up riven auctions of the mock account, including one whose riven stats are unknown. */
export function madeUpAuctions(holdings: Holdings): OwnAuction[] {
  const held = holdings.pieces.flatMap((piece) =>
    piece.detail?.kind === "riven" && piece.detail.revealed
      ? [{ riven: piece.detail.revealed, rank: piece.detail.rank }]
      : [],
  );
  const valued = held.filter(({ riven }) => VALUED.has(WEAPONS[riven.weapon]?.class ?? ""));
  const ways = [
    { startingPrice: 650, buyoutPrice: 650, topBid: null, isDirectSell: true, visible: true },
    { startingPrice: 300, buyoutPrice: 800, topBid: 340, isDirectSell: false, visible: true },
    { startingPrice: 200, buyoutPrice: null, topBid: null, isDirectSell: false, visible: false },
  ];
  const up = valued.slice(0, ways.length).map(({ riven, rank }, at) => ({
    id: `stand-auction-${at}`,
    weapon: riven.weapon,
    riven,
    name: (rivenName(riven) ?? "").toLowerCase(),
    rank,
    created: `2026-10-1${2 - at}T10:00:00.000+00:00`,
    ...ways[at],
  }));
  return [
    ...up,
    {
      id: "stand-auction-unread",
      weapon: null,
      riven: null,
      name: "argi-acritis",
      rank: 0,
      startingPrice: 120,
      buyoutPrice: 120,
      topBid: null,
      isDirectSell: true,
      visible: true,
      created: "2026-10-09T10:00:00.000+00:00",
    },
  ];
}

/** The mock account's appraisable rivens with their ranks, in inventory order. */
function valuedHeld(holdings: Holdings): { riven: Riven; rank: number }[] {
  return holdings.pieces
    .flatMap((piece) =>
      piece.detail?.kind === "riven" && piece.detail.revealed
        ? [{ riven: piece.detail.revealed, rank: piece.detail.rank }]
        : [],
    )
    .filter(({ riven }) => VALUED.has(WEAPONS[riven.weapon]?.class ?? ""));
}

/** Made-up riven auctions the agent picked. */
export function madeUpPicks(holdings: Holdings): RivenPick[] {
  const sellers = [
    ["Vexlorn", "ingame"],
    ["tenno_mika", "online"],
    ["Orvisse", "ingame"],
  ] as const;
  const now = Math.floor(Date.now() / 1000);
  return valuedHeld(holdings)
    .slice(0, 3)
    .map(({ riven, rank }, at) => {
      const [seller, status] = sellers[at] ?? sellers[0];
      const name = (rivenName(riven) ?? "riven").toLowerCase();
      const unrolled = at === 1;
      const buyout = [180, 240, 95][at] ?? 100;
      const worth = [320, 520, 150][at] ?? 150;
      const offer = Math.min(buyout, Math.floor((worth * 0.5) / 5) * 5);
      return {
        id: `stand-pick-${at}`,
        at: now - at * 90,
        name,
        riven: { ...riven, rerolls: unrolled ? 0 : riven.rerolls },
        rank,
        seller,
        status,
        reason: unrolled ? "unrolled" : "cheap",
        buyout,
        worth,
        offer,
        whisper: `/w ${seller} Hi! I'd like to buy your ${name} riven from warframe.market. Would you take ${offer} platinum?`,
      };
    });
}

/** Made-up rivens bought for resale: one held, one sold. */
export function madeUpBuys(holdings: Holdings): RivenBuy[] {
  const now = Math.floor(Date.now() / 1000);
  return valuedHeld(holdings)
    .slice(3, 5)
    .map(({ riven, rank }, at) => ({
      auction: `stand-buy-${at}`,
      name: (rivenName(riven) ?? "riven").toLowerCase(),
      riven,
      rank,
      price: [150, 90][at] ?? 100,
      bought: now - (at + 2) * DAY,
      sold: at === 1 ? now - DAY : null,
      sale: at === 1 ? 160 : null,
    }));
}
