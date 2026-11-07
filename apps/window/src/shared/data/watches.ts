import { type AppState, commands, unwrap, type Watch } from "@median/ipc";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useAppState } from "./app-state";
import { keys } from "./keys";

/** Identity key of a watch: equal watches give equal strings, such as `arrival:baro`. */
export function watchKey(watch: Watch): string {
  switch (watch.kind) {
    case "phase":
      return `phase:${watch.cycle}:${watch.phase}`;
    case "arrival":
      return `arrival:${watch.vendor}`;
    case "fissure":
      return `fissure:${watch.era ?? "*"}:${watch.of ?? "*"}:${watch.mission ?? "*"}`;
  }
}

const NO_WATCHES: readonly Watch[] = [];

/** The user's active watches; empty until the app state loads. */
export function useWatches(): readonly Watch[] {
  return useAppState()?.watches ?? NO_WATCHES;
}

/** Adds or removes a watch; the cache updates at once and rolls back on error. */
export function useWatchToggle() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ watch, on }: { watch: Watch; on: boolean }) =>
      unwrap(commands.setWatch(watch, on)),
    onMutate: async ({ watch, on }) => {
      const before = client.getQueryData<AppState>(keys.state);
      if (!before) return { before };
      const others = before.watches.filter((each) => watchKey(each) !== watchKey(watch));
      client.setQueryData<AppState>(keys.state, {
        ...before,
        watches: on ? [...others, watch] : others,
      });
      return { before };
    },
    onError: (_error, _toggle, context) => {
      if (context?.before) client.setQueryData(keys.state, context.before);
    },
  });
}
