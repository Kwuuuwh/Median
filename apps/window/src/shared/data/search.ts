import { useLocale } from "@/shared/data/app-state";
import { useGoods } from "@/shared/data/catalog";
import { matcher } from "@/shared/lib/search";
import { useKept } from "@/shared/lib/use-kept";

/**
 * Item search that matches the name in either interface language; the typed text is kept under
 * `key` after the page closes.
 */
export function useItemSearch(key: string, items: readonly string[]) {
  const locale = useLocale();
  const [search, setSearch] = useKept(key, "");
  const asked = [...new Set(items)].sort();
  const other = new Map(
    (useGoods(asked, locale === "en" ? "ru" : "en").data ?? []).map((good) => [
      good.item,
      good.name,
    ]),
  );
  const words = matcher(search);
  return {
    /** Value and change handler for the search field. */
    field: { value: search, onChange: setSearch },
    /** Whether the item's name, or its name in the other language, matches the search. */
    matches: (item: string, name: string) => words(`${name} ${other.get(item) ?? ""}`),
  };
}
