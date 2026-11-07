import type { Recipe, SetRow } from "@median/ipc";

/** Items an item card shows: the built item first when the item is in a set, then its parts. */
export type Composition = {
  set: SetRow | null;
  members: string[];
  /** Whether the set is built from the item's own recipe instead of taken from the catalog. */
  fromRecipe: boolean;
};

/** Recipe input classes shown as parts: crafted or acquired items, not gathered resources. */
const KEYED = new Set([
  "component",
  "warframe",
  "primary",
  "secondary",
  "melee",
  "archwing-weapon",
  "companion",
  "companion-weapon",
  "modular",
]);

/** The set an item builds or belongs to, and the items to show for it; for an item in no set, a
 * set made from its own recipe: its blueprint, then its non-resource parts. */
export function compositionOf(
  item: string,
  sets: readonly SetRow[],
  recipe: Recipe | null = null,
  name = "",
): Composition {
  const set =
    sets.find((each) => each.built === item) ??
    sets.find((each) => each.members.includes(item)) ??
    null;
  if (set)
    return {
      set,
      members: [...(set.built ? [set.built] : []), ...set.members],
      fromRecipe: false,
    };
  if (!recipe) return { set: null, members: [item], fromRecipe: false };
  const parts = recipe.parts.filter((part) => KEYED.has(part.class));
  const built: SetRow = {
    slug: "",
    name,
    ducats: null,
    vaulted: null,
    built: item,
    members: [recipe.blueprint, ...parts.map((part) => part.item)],
    per_set: [1, ...parts.map((part) => part.count)],
  };
  return { set: built, members: [item, ...built.members], fromRecipe: true };
}

/** A part's name without the leading set name; the full name when nothing would be left. */
export function partName(name: string, setName: string): string {
  if (!setName || !name.startsWith(setName)) return name;
  const rest = name
    .slice(setName.length)
    .replace(/^[:\s]+/u, "")
    .replace(/^\((.*)\)$/u, "$1");
  return rest || name;
}
