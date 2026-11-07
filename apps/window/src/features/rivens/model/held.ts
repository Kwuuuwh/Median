import type { Piece, Riven } from "@median/ipc";

/** Riven class: the weapon type a riven's stats apply to. */
export type RivenClass =
  | "rifle"
  | "shotgun"
  | "pistol"
  | "melee"
  | "archgun"
  | "kitgun"
  | "zaw"
  | "companion";

/** One revealed riven the account holds. */
export type HeldRiven = {
  /** Key unique among the account's rivens: item key and variant. */
  key: string;
  item: string;
  class: RivenClass;
  rank: number;
  riven: Riven;
};

/** Number of veiled rivens of one class the account holds. */
export type VeiledCount = { class: RivenClass; count: number };

/** Classes in page order. */
const CLASSES: readonly RivenClass[] = [
  "rifle",
  "shotgun",
  "pistol",
  "melee",
  "archgun",
  "kitgun",
  "zaw",
  "companion",
];

/** Item key fragment of each riven class, checked in order: `ModularPistol` before `Pistol`. */
const CLASS_WORDS: readonly [string, RivenClass][] = [
  ["SentinelWeapon", "companion"],
  ["ModularPistol", "kitgun"],
  ["ModularMelee", "zaw"],
  ["Archgun", "archgun"],
  ["Shotgun", "shotgun"],
  ["Pistol", "pistol"],
  ["Rifle", "rifle"],
  ["Melee", "melee"],
];

const RIVENS = "/Lotus/Upgrades/Mods/Randomized/";

/** Highest rank a riven reaches. */
export const RIVEN_TOP_RANK = 8;

/** Riven class of an item key; null when the key is not a riven's. */
export function classOf(item: string): RivenClass | null {
  if (!item.startsWith(RIVENS)) return null;
  const name = item.slice(RIVENS.length);
  return CLASS_WORDS.find(([word]) => name.includes(word))?.[1] ?? null;
}

/** Rivens in the inventory: revealed ones individually, veiled ones counted per class. */
export function heldRivens(pieces: readonly Piece[]): {
  revealed: HeldRiven[];
  veiled: VeiledCount[];
} {
  const revealed: HeldRiven[] = [];
  const veiled = new Map<RivenClass, number>();
  for (const piece of pieces) {
    const kind = classOf(piece.item);
    if (!kind || piece.count === 0) continue;
    const detail = piece.detail;
    if (detail?.kind === "riven" && detail.revealed) {
      revealed.push({
        key: `${piece.item}|${piece.variant}`,
        item: piece.item,
        class: kind,
        rank: detail.rank,
        riven: detail.revealed,
      });
    } else {
      veiled.set(kind, (veiled.get(kind) ?? 0) + piece.count);
    }
  }
  return {
    revealed,
    veiled: CLASSES.flatMap((kind) => {
      const count = veiled.get(kind);
      return count ? [{ class: kind, count }] : [];
    }),
  };
}
