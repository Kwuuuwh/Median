import { useSyncExternalStore } from "react";

/** The item whose card is open, with the held variant it was opened from; null when closed. */
let opened: { item: string; variant: string | null } | null = null;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Opens an item's card over the window at the given held variant; `null` closes the card. */
export function openItem(item: string | null, variant: string | null = null): void {
  opened = item === null ? null : { item, variant };
  for (const listener of listeners) listener();
}

/** The open item and its variant, re-rendering the caller on change; null while no card is open. */
export function useOpenItem(): { item: string; variant: string | null } | null {
  return useSyncExternalStore(subscribe, () => opened);
}
