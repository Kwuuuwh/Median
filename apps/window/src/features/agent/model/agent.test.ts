import type { AppState, OwnOrder, Policy } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { agentBids, blockersOf, budgetOf, tiedBy } from "./agent";

const policy = { mostTied: 300, tiedFromPurse: false, purseShare: 50 } as Policy;

function order(byAgent: boolean, selling: boolean, platinum: number, quantity = 1): OwnOrder {
  return { byAgent, selling, platinum, quantity } as OwnOrder;
}

describe("the agent's bids", () => {
  it("are the buys it put up itself", () => {
    const orders = [order(true, false, 40), order(true, true, 90), order(false, false, 20)];
    expect(agentBids(orders)).toHaveLength(1);
    expect(tiedBy(agentBids(orders))).toBe(40);
  });

  it("tie up every copy they ask for", () => {
    expect(tiedBy([order(true, false, 40, 3)])).toBe(120);
  });
});

describe("the budget", () => {
  it("is the person's number unless it is read from the purse", () => {
    expect(budgetOf(policy, 1_000)).toBe(300);
  });

  it("is a share of the purse, which the bids do not take platinum out of", () => {
    expect(budgetOf({ ...policy, tiedFromPurse: true }, 404)).toBe(202);
    expect(budgetOf({ ...policy, tiedFromPurse: true }, null)).toBeNull();
  });
});

describe("what keeps the agent from bidding", () => {
  const ready = {
    membership: { standing: "in" },
    account: { platinum: 400, exchanges: 10 },
    policy: { leastExchanges: 0 },
  } as AppState;

  it("is nothing once signed in with a purse, a budget and exchanges", () => {
    expect(blockersOf(ready, 200)).toEqual([]);
  });

  it("is every bound that is not met", () => {
    const stuck = {
      ...ready,
      membership: { standing: "out" },
      account: { platinum: null, exchanges: 0 },
    } as AppState;
    expect(blockersOf(stuck, 0)).toEqual(["signed-out", "purse", "budget", "exchanges"]);
  });
});
