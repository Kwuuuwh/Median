import {
  type Curve,
  commands,
  type Orders,
  type OwnOrder,
  type Pair,
  type Priced,
  type Quote,
  unwrap,
} from "@median/ipc";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useLocale } from "./app-state";
import { keys } from "./keys";

/** Map key of a holding's price: item and variant joined by a NUL character. */
export function priceKey(item: string, variant: string): string {
  return `${item}\u0000${variant}`;
}

/** Market cache prices of every holding, keyed by `priceKey`. */
export function usePrices() {
  return useQuery<Map<string, Priced>>({
    queryKey: keys.prices,
    queryFn: async () => {
      const prices = await unwrap(commands.getPrices());
      return new Map(prices.map((price) => [priceKey(price.item, price.variant), price]));
    },
  });
}

/** Market cache prices of held sets, keyed by market slug. */
export function useSetPrices() {
  return useQuery<Map<string, Priced>>({
    queryKey: keys.setPrices,
    queryFn: async () => {
      const prices = await unwrap(commands.getSetPrices());
      return new Map(prices.map((price) => [price.slug, price]));
    },
  });
}

/** Refetch interval of the account's orders, so changes made outside the app appear. */
const ORDERS_REFRESH_MS = 60_000;

/** Retries of a failed read of the account's orders before the query reports the error. */
const ORDERS_RETRIES = 3;

/** The account's open orders on warframe.market, in the interface language. */
export function useOwnOrders() {
  const lang = useLocale();
  return useQuery<OwnOrder[]>({
    queryKey: keys.orders(lang),
    queryFn: () => unwrap(commands.getOrders(lang)),
    refetchInterval: ORDERS_REFRESH_MS,
    retry: ORDERS_RETRIES,
  });
}

/**
 * All open orders of one holding, fetched from warframe.market on each mount and every `every` ms
 * when given; not kept in the cache after unmount.
 */
export function useListings(item: string, variant: string, every?: number) {
  return useQuery<Orders>({
    queryKey: keys.listings(item, variant),
    queryFn: () => unwrap(commands.getListings(item, variant)),
    staleTime: 0,
    gcTime: 0,
    refetchInterval: every,
  });
}

/**
 * Market history of one holding over the last `days`: closed trades and both order sides, at the
 * top rank when `topRank`; the previous curve stays shown while the next loads.
 */
export function useCurve(item: string, variant: string, days: number, topRank = false) {
  return useQuery<Curve>({
    queryKey: keys.curve(item, variant, days, topRank),
    placeholderData: keepPreviousData,
    queryFn: () => unwrap(commands.getCurve(item, variant, days, topRank)),
  });
}

/** Market price of each item in platinum; items without a price are omitted. */
export function useQuotes(items: readonly string[]) {
  return useQuery<Quote[], Error, Map<string, number>>({
    queryKey: keys.quotes(items),
    queryFn: () => unwrap(commands.getQuotes([...items])),
    select: (quotes) =>
      new Map(
        quotes.flatMap((quote) =>
          quote.platinum === null ? [] : [[quote.item, quote.platinum] as const],
        ),
      ),
    enabled: items.length > 0,
  });
}

/** Market pair of each item listed on warframe.market; unlisted items are omitted. */
export function usePairs(items: readonly string[]) {
  return useQuery<Quote[], Error, Map<string, Pair>>({
    queryKey: keys.quotes(items),
    queryFn: () => unwrap(commands.getQuotes([...items])),
    select: (quotes) =>
      new Map(quotes.flatMap((quote) => (quote.pair ? [[quote.item, quote.pair] as const] : []))),
    enabled: items.length > 0,
  });
}
