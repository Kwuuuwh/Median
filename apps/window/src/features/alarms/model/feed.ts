import type { Alarm } from "@median/ipc";
import { useSyncExternalStore } from "react";

/** A feed entry: a triggered alarm with its feed id and Unix second `at` it triggered. */
type Rung = { id: number; at: number; alarm: Alarm };

/** Maximum number of alarms the feed keeps; older ones are dropped. */
const KEPT = 50;

type Feed = { rung: readonly Rung[]; unseen: number };

let feed: Feed = { rung: [], unseen: 0 };
let nextId = 0;
const listeners = new Set<() => void>();

function publish(next: Feed): void {
  feed = next;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Adds alarms to the top of the feed, newest first, and counts them as unseen. */
export function recordAlarms(alarms: readonly Alarm[], at: number): void {
  const added = alarms.map((alarm) => ({ id: nextId++, at, alarm }));
  publish({
    rung: [...added.reverse(), ...feed.rung].slice(0, KEPT),
    unseen: feed.unseen + added.length,
  });
}

/** Marks every alarm in the feed as seen. */
export function markSeen(): void {
  if (feed.unseen > 0) publish({ ...feed, unseen: 0 });
}

/** Removes one alarm from the feed. */
export function dismissAlarm(id: number): void {
  publish({ ...feed, rung: feed.rung.filter((each) => each.id !== id) });
}

/** Empties the feed. */
export function clearFeed(): void {
  publish({ rung: [], unseen: 0 });
}

/** This session's alarms, newest first, and the number of unseen ones. */
export function currentFeed(): Feed {
  return feed;
}

/** The alarm feed; re-renders the caller on every change. */
export function useFeed(): Feed {
  return useSyncExternalStore(subscribe, currentFeed);
}
