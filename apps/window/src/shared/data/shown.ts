import { commands, type Pair } from "@median/ipc";
import { useEffect } from "react";

/** Pairs on screen, per registering `useShown` call. */
const screen = new Map<symbol, readonly Pair[]>();
let opened: Pair | null = null;

/** Sends the deduplicated on-screen pairs to the backend, which refreshes their prices first. */
function declare(): void {
  const seen = new Set<string>();
  const shown = [...screen.values()].flat().filter((pair) => {
    const key = `${pair.slug}:${pair.key}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  void commands.setShown(shown, opened);
}

/** Registers pairs as shown, for market cache refresh priority, while the caller is mounted. */
export function useShown(pairs: readonly Pair[]): void {
  const key = JSON.stringify(pairs);
  // biome-ignore lint/correctness/useExhaustiveDependencies: pairs are tracked by their JSON key
  useEffect(() => {
    const id = Symbol();
    screen.set(id, pairs);
    declare();
    return () => {
      screen.delete(id);
      declare();
    };
  }, [key]);
}

/** Registers the pair of the open item card, which the market cache refreshes before all others. */
export function useOpened(pair: Pair | null): void {
  const key = JSON.stringify(pair);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the pair is tracked by its JSON key
  useEffect(() => {
    opened = pair;
    declare();
    return () => {
      opened = null;
      declare();
    };
  }, [key]);
}
