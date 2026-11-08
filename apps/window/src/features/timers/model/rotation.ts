import type { Good, Offer, Relic, SetRow } from "@median/ipc";
import { partName } from "@/features/items/model/composition";
import { isSettled, type Standing } from "@/features/items/model/standing";
import { nameCollator } from "@/shared/lib/sort";

/** A relic a part drops from, with the part's reward rarity in it. */
type Drop = { base: string; name: string; rarity: string };

/** One part of an offered set. */
type SetPart = {
  item: string;
  /** The part's name without the set's name. */
  name: string;
  image: string | null;
  own: number;
  /** Copies one set needs. */
  need: number;
  platinum: number | null;
  drops: Drop[];
};

/** A prime set in the vendor's current rotation. */
export type PrimeSet = {
  built: string;
  name: string;
  image: string | null;
  kind: string;
  standing: Standing;
  /** Market price of the complete set in platinum. */
  platinum: number | null;
  parts: SetPart[];
};

/** Inputs the rotation is built from. */
export type RotationSources = {
  offers: readonly Offer[];
  sets: readonly SetRow[];
  relics: readonly Relic[];
  goods: ReadonlyMap<string, Good>;
  held: ReadonlyMap<string, number>;
  quotes: ReadonlyMap<string, number>;
  /** Position of each kind's class in the catalog order. */
  order: (kind: string) => number;
  locale: string;
};

/**
 * Offered prime sets with a part dropping from an offered relic, with their parts and the relics
 * that drop them; sets neither held nor mastered first, then by class order and name.
 */
export function rotationOf(sources: RotationSources): PrimeSet[] {
  const drops = new Map<string, Drop[]>();
  const slots = new Map<string, Relic["holds"][number]>();
  for (const relic of sources.relics) {
    for (const slot of relic.holds) {
      slots.set(slot.item, slot);
      drops.set(slot.item, [
        ...(drops.get(slot.item) ?? []),
        { base: relic.base, name: relic.name, rarity: slot.rarity },
      ]);
    }
  }
  const byBuilt = new Map(sources.sets.flatMap((set) => (set.built ? [[set.built, set]] : [])));
  const collator = nameCollator(sources.locale);

  const rotation = sources.offers.flatMap((offer): PrimeSet[] => {
    const set = byBuilt.get(offer.item);
    const good = sources.goods.get(offer.item);
    if (!set || !good) return [];
    const parts = set.members.map((member, at): SetPart => {
      const slot = slots.get(member);
      const part = sources.goods.get(member);
      return {
        item: member,
        name: partName(slot?.name ?? part?.name ?? member, good.name),
        image: slot?.image ?? part?.image ?? null,
        own: slot?.own ?? sources.held.get(member) ?? 0,
        need: set.per_set[at] ?? 1,
        platinum: slot?.platinum ?? sources.quotes.get(member) ?? null,
        drops: drops.get(member) ?? [],
      };
    });
    if (parts.every((part) => part.drops.length === 0)) return [];
    return [
      {
        built: offer.item,
        name: good.name,
        image: good.image,
        kind: good.kind,
        standing: {
          held: sources.held.get(offer.item) ?? 0,
          mastered: good.mastered === true,
          parts: {
            owned: parts.filter((part) => part.own >= part.need).length,
            total: parts.length,
          },
        },
        platinum: sources.quotes.get(offer.item) ?? null,
        parts,
      },
    ];
  });

  return rotation.sort(
    (a, b) =>
      Number(isSettled(a.standing)) - Number(isSettled(b.standing)) ||
      sources.order(a.kind) - sources.order(b.kind) ||
      collator.compare(a.name, b.name),
  );
}

/** Parts held below the count one set needs, from sets the account neither holds nor mastered. */
export function wantedParts(rotation: readonly PrimeSet[]): Set<string> {
  return new Set(
    rotation.flatMap((set) =>
      isSettled(set.standing)
        ? []
        : set.parts.filter((part) => part.own < part.need).map((part) => part.item),
    ),
  );
}
