import recorded from "virtual:median-snapshot";
import type { Collectible } from "@median/ipc";
import { goodsIn } from "./snapshot";

/**
 * Catalog row of `item`, named and pictured as in the snapshot, of one held copy that is not
 * mastered, with `changes` over it.
 */
export function sampleCollectible(item: string, changes: Partial<Collectible> = {}): Collectible {
  const lang = recorded.getState?.[0]?.answer.locale ?? "ru";
  const good = goodsIn(recorded, lang).get(item);
  const kinds = recorded.getTaxonomy?.[0]?.answer.kinds ?? [];
  return {
    item,
    name: good?.name ?? item.slice(item.lastIndexOf("/") + 1),
    image: good?.image ?? null,
    class: kinds.find((kind) => kind.slug === good?.kind)?.class ?? "",
    kind: good?.kind ?? "",
    prime: good?.prime ?? false,
    vaulted: good?.vaulted ?? false,
    tradable: good?.tradable ?? false,
    owned: 1,
    mastered: false,
    ready: null,
    required: 0,
    reachable: true,
    relics: false,
    ...changes,
  };
}
