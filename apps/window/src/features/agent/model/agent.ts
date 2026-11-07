import type { AppState, OwnOrder, Policy } from "@median/ipc";

/** The agent's open buy orders. */
export function agentBids(orders: readonly OwnOrder[]): OwnOrder[] {
  return orders.filter((order) => order.byAgent && !order.selling);
}

/** Platinum committed by the buy orders: price times quantity, summed. */
export function tiedBy(bids: readonly OwnOrder[]): number {
  return bids.reduce((sum, bid) => sum + bid.platinum * bid.quantity, 0);
}

/**
 * Platinum the agent may commit: the user's fixed limit, or `purseShare` percent (at most 100) of
 * the platinum held; null while the platinum held is unknown.
 */
export function budgetOf(policy: Policy, purse: number | null): number | null {
  if (!policy.tiedFromPurse) return policy.mostTied;
  if (purse === null) return null;
  return Math.floor((purse * Math.min(policy.purseShare, 100)) / 100);
}

/** A reason the agent cannot place a buy order. */
export type Blocker = "signed-out" | "purse" | "budget" | "exchanges";

/** Every current reason the agent cannot place buy orders; empty when it can. */
export function blockersOf(state: AppState, budget: number | null): Blocker[] {
  const { account, policy, membership } = state;
  const blockers: Blocker[] = [];
  if (membership.standing !== "in") blockers.push("signed-out");
  if (account.platinum === null) blockers.push("purse");
  if (budget === 0) blockers.push("budget");
  if (account.exchanges !== null && account.exchanges <= policy.leastExchanges)
    blockers.push("exchanges");
  return blockers;
}
