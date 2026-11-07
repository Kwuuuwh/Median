import {
  commands,
  type OwnAuction,
  type RerollOdds,
  type Riven,
  type RivenAppraisal,
  type RivenBand,
  type RivenBuy,
  type RivenChoices,
  type RivenDemand,
  unwrap,
} from "@median/ipc";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useAppState, useLocale } from "./app-state";
import { keys } from "./keys";

/**
 * Appraisal of each riven at its rank on its weapon: per stat, overall, and reroll odds, in input
 * order; an entry is undefined while it loads.
 */
export function useRivenAppraisals(rivens: readonly { riven: Riven; rank: number }[]) {
  const lang = useLocale();
  return useQueries({
    queries: rivens.map(({ riven, rank }) => ({
      queryKey: keys.rivenAppraisal(lang, JSON.stringify(riven), rank),
      queryFn: (): Promise<RivenAppraisal | null> =>
        unwrap(commands.getRivenAppraisal(lang, riven, rank)),
      staleTime: Number.POSITIVE_INFINITY,
    })),
    combine: (answers) => answers.map((answer) => answer.data),
  });
}

/** In-game name of each riven, in input order; null where it cannot be determined. */
export function useRivenNames(rivens: readonly Riven[]) {
  return useQuery<(string | null)[]>({
    queryKey: keys.rivenNames(rivens.map((riven) => JSON.stringify(riven))),
    queryFn: () => unwrap(commands.getRivenNames([...rivens])),
    enabled: rivens.length > 0,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

/** Seller asking prices of rivens similar to each one, in input order; undefined while loading. */
export function useRivenBands(rivens: readonly Riven[]) {
  return useQueries({
    queries: rivens.map((riven) => ({
      queryKey: keys.rivenBand(JSON.stringify(riven)),
      queryFn: (): Promise<RivenBand | null> => unwrap(commands.getRivenBand(riven)),
      staleTime: 60 * 60 * 1000,
    })),
    combine: (answers) => answers.map((answer) => answer.data),
  });
}

/** Highest price of a rolled riven of each weapon, in input order; undefined while loading. */
export function useRivenCeilings(weapons: readonly string[]) {
  return useQueries({
    queries: weapons.map((weapon) => ({
      queryKey: keys.rivenCeiling(weapon),
      queryFn: (): Promise<number | null> => unwrap(commands.getRivenCeiling(weapon)),
      staleTime: 60 * 60 * 1000,
    })),
    combine: (answers) => answers.map((answer) => answer.data),
  });
}

/**
 * How readily rivens of each weapon sell, separately for rolled and unrolled ones, in input order;
 * an entry is undefined while it loads.
 */
export function useRivenDemands(rivens: readonly Riven[]) {
  return useQueries({
    queries: rivens.map((riven) => ({
      queryKey: keys.rivenDemand(riven.weapon, riven.rerolls > 0),
      queryFn: (): Promise<RivenDemand | null> =>
        unwrap(commands.getRivenDemand(riven.weapon, riven.rerolls > 0)),
      staleTime: 60 * 60 * 1000,
    })),
    combine: (answers) => answers.map((answer) => answer.data),
  });
}

/** Value of a riven when sold and when transmuted, with similar auctions; null without a riven. */
export function useRivenChoices(riven: Riven | null) {
  const lang = useLocale();
  return useQuery<RivenChoices | null>({
    queryKey: keys.rivenChoices(lang, JSON.stringify(riven)),
    queryFn: () => (riven ? unwrap(commands.getRivenChoices(lang, riven)) : null),
    enabled: riven !== null,
    staleTime: 60 * 60 * 1000,
  });
}

/**
 * Per locked stat, the chance that a reroll of the riven rolls better and the price of such rolls;
 * null without a riven.
 */
export function useRivenRolls(riven: Riven | null) {
  return useQuery<RerollOdds[] | null>({
    queryKey: keys.rivenRolls(JSON.stringify(riven)),
    queryFn: () => (riven ? unwrap(commands.getRivenRolls(riven)) : null),
    enabled: riven !== null,
    staleTime: 60 * 60 * 1000,
  });
}

/** Opens a riven auction's warframe.market page in the browser. */
export function openAuction(id: string) {
  void unwrap(commands.openAuction(id));
}

/** The account's open riven auctions; disabled while signed out. */
export function useOwnAuctions() {
  const signedIn = useAppState()?.membership.standing === "in";
  return useQuery<OwnAuction[]>({
    queryKey: keys.ownAuctions,
    queryFn: () => unwrap(commands.getOwnAuctions()),
    enabled: signedIn,
  });
}

/** A mutation of own riven auctions that refetches them once it settles. */
function useAuctionChange<V, R>(run: (vars: V) => Promise<R>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSettled: () => client.invalidateQueries({ queryKey: keys.ownAuctions }),
  });
}

/** Lists a riven for auction. */
export function usePutUpRiven() {
  return useAuctionChange(
    ({
      riven,
      rank,
      starting,
      buyout,
    }: {
      riven: Riven;
      rank: number;
      starting: number;
      buyout: number | null;
    }) => unwrap(commands.putUpRiven(riven, rank, starting, buyout)),
  );
}

/** Changes one riven auction's prices and visibility. */
export function useChangeAuction() {
  return useAuctionChange(
    ({
      id,
      starting,
      buyout,
      visible,
    }: {
      id: string;
      starting: number;
      buyout: number | null;
      visible: boolean;
    }) => unwrap(commands.changeRivenAuction(id, starting, buyout, visible)),
  );
}

/** Closes one riven auction. */
export function useCloseAuction() {
  return useAuctionChange((id: string) => unwrap(commands.closeRivenAuction(id)));
}

/** Rivens bought for resale, newest first. */
export function useRivenBuys() {
  return useQuery<RivenBuy[]>({
    queryKey: keys.rivenBuys,
    queryFn: () => unwrap(commands.getRivenBuys()),
  });
}

/** A mutation of the riven buys that refetches them once it settles. */
function useBuysChange<V, R>(run: (vars: V) => Promise<R>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSettled: () => client.invalidateQueries({ queryKey: keys.rivenBuys }),
  });
}

/** Records the purchase of a riven the agent picked, at `price` platinum. */
export function useRecordRivenBuy() {
  return useBuysChange(({ id, price }: { id: string; price: number }) =>
    unwrap(commands.recordRivenBuy(id, price)),
  );
}

/** Records the sale of a bought riven at `price` platinum. */
export function useRecordRivenSale() {
  return useBuysChange(({ auction, price }: { auction: string; price: number }) =>
    unwrap(commands.recordRivenSale(auction, price)),
  );
}

/** Dismisses a riven the agent picked to buy. */
export function useDismissRivenPick() {
  return useMutation({ mutationFn: (id: string) => unwrap(commands.dismissRivenPick(id)) });
}
