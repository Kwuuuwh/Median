import {
  type AppState,
  type Catalog,
  events,
  type Game,
  type Holdings,
  type Piece,
  type Priced,
  type Quote,
} from "@median/ipc";
import type { QueryClient } from "@tanstack/react-query";
import { keys } from "@/shared/data/keys";
import { priceKey } from "@/shared/data/market";

/** Holdings at the new `step` with the moved pieces applied; a zero count removes a piece. */
export function applyMoves(
  holdings: Holdings,
  step: number,
  digest: string,
  moved: Piece[],
): Holdings {
  const byKey = new Map(
    holdings.pieces.map((piece) => [priceKey(piece.item, piece.variant), piece]),
  );
  for (const piece of moved) {
    const key = priceKey(piece.item, piece.variant);
    if (piece.count === 0) byKey.delete(key);
    else byKey.set(key, piece);
  }
  return { step, digest, pieces: [...byKey.values()] };
}

/** Whether the catalog or its pictures changed while the cached state was ready. */
export function dataReplaced(before: AppState | undefined, catalog: Catalog): boolean {
  if (!before?.catalog.ready || !catalog.ready) return false;
  return before.catalog.installed !== catalog.installed || before.catalog.pack !== catalog.pack;
}

/** Whether the newest exchange the game confirmed is missing from the cached app state. */
export function traded(before: AppState | undefined, game: Game): boolean {
  const last = game.swaps[0];
  return last !== undefined && last.at !== before?.game.swaps[0]?.at;
}

/** Whether any of `quotes` is priced from the market pair of one of `slugs`. */
export function quotesFrom(
  quotes: readonly Quote[] | undefined,
  slugs: ReadonlySet<string>,
): boolean {
  return quotes?.some((quote) => quote.pair !== null && slugs.has(quote.pair.slug)) ?? false;
}

/** Cached responses built on the market cache's closed trades and item descriptions. */
const PRICED = [
  keys.prices,
  keys.setPrices,
  keys.quotesAll,
  keys.relicsAll,
  keys.relicsOfAll,
  keys.syndicatesAll,
  keys.haulAll,
  keys.roadmapAll,
  keys.curveAll,
] as const;

/** Refetches what the changed closed trades or descriptions of `slugs` price; all of it for null. */
function requote(client: QueryClient, slugs: readonly string[] | null): void {
  if (slugs === null) {
    for (const queryKey of PRICED) void client.invalidateQueries({ queryKey });
    return;
  }
  const moved = new Set(slugs);
  void client.invalidateQueries({
    queryKey: keys.quotesAll,
    predicate: (query) => quotesFrom(query.state.data as Quote[] | undefined, moved),
  });
  const held = client.getQueryData<Map<string, Priced>>(keys.prices);
  if ([...(held?.values() ?? [])].some((row) => moved.has(row.slug))) {
    void client.invalidateQueries({ queryKey: keys.prices });
  }
  const sets = client.getQueryData<Map<string, Priced>>(keys.setPrices);
  if (slugs.some((slug) => sets?.has(slug))) {
    void client.invalidateQueries({ queryKey: keys.setPrices });
  }
}

function patchState(client: QueryClient, patch: Partial<AppState>): void {
  client.setQueryData<AppState>(keys.state, (state) => (state ? { ...state, ...patch } : state));
}

function patchPrices(
  client: QueryClient,
  key: readonly string[],
  rows: Priced[],
  by: (row: Priced) => string,
): void {
  if (rows.length === 0) return;
  client.setQueryData<Map<string, Priced>>(key, (table) => {
    if (!table) return table;
    const next = new Map(table);
    for (const row of rows) next.set(by(row), row);
    return next;
  });
}

/** Subscribes to backend events and patches or invalidates the matching query cache entries. */
export async function followEvents(client: QueryClient): Promise<void> {
  await Promise.all([
    events.settingsChanged.listen(({ payload }) => patchState(client, payload)),
    events.accountChanged.listen(({ payload }) => patchState(client, payload)),
    events.collectorChanged.listen(({ payload }) => patchState(client, payload)),
    events.tasksChanged.listen(({ payload }) => patchState(client, payload)),
    events.quarantineChanged.listen(({ payload }) => patchState(client, payload)),
    events.membershipChanged.listen(({ payload }) => patchState(client, payload)),
    events.catalogChanged.listen(({ payload }) => {
      const before = client.getQueryData<AppState>(keys.state);
      patchState(client, payload);
      if (!dataReplaced(before, payload.catalog)) return;
      void client.invalidateQueries({ predicate: (query) => query.queryKey[0] !== keys.state[0] });
    }),
    events.gameChanged.listen(({ payload }) => {
      const before = client.getQueryData<AppState>(keys.state);
      patchState(client, payload);
      if (!traded(before, payload.game)) return;
      void client.invalidateQueries({ queryKey: keys.figuresAll });
      void client.invalidateQueries({ queryKey: keys.positionsAll });
      void client.invalidateQueries({ queryKey: keys.ordersAll });
    }),
    events.agentChanged.listen(({ payload }) => patchState(client, payload)),
    events.policyChanged.listen(({ payload }) => {
      patchState(client, payload);
      void client.invalidateQueries({ queryKey: keys.relicsAll });
      void client.invalidateQueries({ queryKey: keys.relicsOfAll });
      void client.invalidateQueries({ queryKey: keys.syndicatesAll });
    }),
    events.worldChanged.listen(({ payload }) => patchState(client, payload)),
    events.marketChanged.listen(({ payload }) => {
      patchState(client, payload);
      void client.invalidateQueries({ queryKey: keys.ordersAll });
    }),
    events.inventoryChanged.listen(({ payload }) => {
      void client.invalidateQueries({ queryKey: keys.relicsAll });
      void client.invalidateQueries({ queryKey: keys.syndicatesAll });
      void client.invalidateQueries({ queryKey: keys.haulAll });
      void client.invalidateQueries({ queryKey: keys.masteryAll });
      void client.invalidateQueries({ queryKey: keys.roadmapAll });
      void client.invalidateQueries({ queryKey: keys.collectiblesAll });
      const holdings = client.getQueryData<Holdings>(keys.holdings);
      if (!holdings || payload.step !== holdings.step + 1) {
        void client.invalidateQueries({ queryKey: keys.holdings });
        return;
      }
      client.setQueryData(
        keys.holdings,
        applyMoves(holdings, payload.step, payload.digest, payload.moved),
      );
    }),
    events.bookMoved.listen(() => {
      void client.invalidateQueries({ queryKey: keys.ordersAll });
    }),
    events.quotesChanged.listen(({ payload }) => requote(client, payload.slugs)),
    events.pricesChanged.listen(({ payload }) => {
      patchPrices(client, keys.prices, payload.held, (row) => priceKey(row.item, row.variant));
      patchPrices(client, keys.setPrices, payload.sets, (row) => row.slug);
      if (payload.held.length > 0) void client.invalidateQueries({ queryKey: keys.haulAll });
    }),
  ]);
}
