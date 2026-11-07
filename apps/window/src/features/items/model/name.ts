/** A readable name for an item missing from the catalog, made from the last segment of its key. */
export function nameFromKey(item: string): string {
  const last = item.split("/").at(-1) ?? item;
  return last
    .replace(/^MPV(?=[A-Z])/, "")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .replace(/([A-Za-z])(\d)/g, "$1 $2");
}
