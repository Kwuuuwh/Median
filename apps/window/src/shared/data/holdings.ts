import { commands, type Holdings } from "@median/ipc";
import { useQuery } from "@tanstack/react-query";
import { keys } from "./keys";

/** The account's full inventory. */
export function useHoldings() {
  return useQuery<Holdings>({ queryKey: keys.holdings, queryFn: () => commands.getInventory() });
}
