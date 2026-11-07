/** A relic's name without the word for relic: `Лит K5`, `Lith K5`. */
export function relicLabel(name: string): string {
  return name.replace(/^Реликвия\s+/u, "").replace(/\s+Relic$/u, "");
}
