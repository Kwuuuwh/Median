import {
  type Card,
  type Collectible,
  commands,
  type GameData,
  type Good,
  type Node,
  type Relic,
  type SetRow,
  type Taxonomy,
  unwrap,
} from "@median/ipc";
import { keepPreviousData, type QueryClient, queryOptions, useQuery } from "@tanstack/react-query";
import { useLocale } from "./app-state";
import { keys } from "./keys";

/** Catalog rows of these items in the interface language, or in `asked` when given. */
export function useGoods(items: readonly string[], asked?: string) {
  const locale = useLocale();
  const lang = asked ?? locale;
  return useQuery<Good[]>({
    queryKey: keys.goods(lang, items),
    queryFn: () => unwrap(commands.getGoods(lang, [...items])),
    enabled: items.length > 0,
  });
}

/** Item classes and kinds of the catalog, in the interface language. */
export function useTaxonomy() {
  const lang = useLocale();
  return useQuery<Taxonomy>({
    queryKey: keys.taxonomy(lang),
    queryFn: () => unwrap(commands.getTaxonomy(lang)),
  });
}

/** The open game data's version, date and game build. */
export function useGameData() {
  return useQuery<GameData>({
    queryKey: keys.gameData,
    queryFn: () => unwrap(commands.getGameData()),
  });
}

/** Every item the catalog page lists, with the account's state of each, in the interface language. */
export function useCollectibles() {
  const lang = useLocale();
  return useQuery<Collectible[]>({
    queryKey: keys.collectibles(lang),
    queryFn: () => unwrap(commands.getCatalogue(lang)),
    placeholderData: keepPreviousData,
  });
}

/** All sets in the catalog, in the interface language. */
export function useSets() {
  const lang = useLocale();
  return useQuery<SetRow[]>({
    queryKey: keys.sets(lang),
    queryFn: () => unwrap(commands.getSets(lang)),
  });
}

/** The item's name in the other interface language (ru or en); null when it has none. */
export function useOtherName(item: string) {
  const lang = useLocale();
  return useQuery<string | null>({
    queryKey: keys.otherName(lang, item),
    queryFn: () => unwrap(commands.getOtherName(lang, item)),
  });
}

/**
 * Query options of these star chart nodes, mapped by node key; `useNodes` and `fetchNodes` share
 * them, so components and event handlers use one cache entry.
 */
function nodesQuery(nodeKeys: readonly string[]) {
  return queryOptions<Map<string, Node>>({
    queryKey: keys.nodes(nodeKeys),
    queryFn: async () =>
      new Map((await unwrap(commands.getNodes([...nodeKeys]))).map((node) => [node.node, node])),
  });
}

/** Names, planets and missions of these star chart nodes, by node key. */
export function useNodes(nodeKeys: readonly string[]) {
  return useQuery({ ...nodesQuery(nodeKeys), enabled: nodeKeys.length > 0 });
}

/** `useNodes` outside components: served from the cache when present; empty for no keys. */
export function fetchNodes(
  client: QueryClient,
  nodeKeys: readonly string[],
): Promise<Map<string, Node>> {
  return nodeKeys.length > 0 ? client.fetchQuery(nodesQuery(nodeKeys)) : Promise.resolve(new Map());
}

/** Squad size the average platinum and ducats of a relic run are computed for. */
export const SQUAD = 4;

/** Relics the account holds, with the average platinum and ducats per run of each refinement. */
export function useRelics(squad = SQUAD, placeholderData?: typeof keepPreviousData) {
  const lang = useLocale();
  return useQuery<Relic[]>({
    queryKey: keys.relics(lang, squad),
    queryFn: () => unwrap(commands.getRelics(lang, squad)),
    placeholderData,
  });
}

/** Relics of these item keys with their average run value, mapped by each refinement's item key. */
export function useRelicsOf(items: readonly string[]) {
  const lang = useLocale();
  return useQuery<Map<string, Relic>>({
    queryKey: keys.relicsOf(lang, items),
    queryFn: async () => {
      const relics = await unwrap(commands.getRelicsOf(lang, [...items], SQUAD));
      return new Map(
        relics.flatMap((relic) => relic.steps.map((step) => [step.item, relic] as const)),
      );
    },
    enabled: items.length > 0,
  });
}

/**
 * Catalog card of one item: components, drop sources and vendors; the previous card stays shown
 * while the next loads.
 */
export function useCard(item: string | null) {
  const lang = useLocale();
  return useQuery<Card | null>({
    queryKey: keys.card(lang, item ?? ""),
    placeholderData: keepPreviousData,
    queryFn: () => unwrap(commands.getCard(lang, item ?? "")),
    enabled: item !== null,
  });
}
