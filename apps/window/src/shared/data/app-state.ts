import { type AppState, commands } from "@median/ipc";
import { useQuery } from "@tanstack/react-query";
import { keys } from "./keys";

/** App state from the backend, kept current by its events; undefined until the first response. */
export function useAppState(): AppState | undefined {
  return useQuery({ queryKey: keys.state, queryFn: () => commands.getState() }).data;
}

/** Why this run did not start; null after a good start, undefined until the answer comes. */
export function useStartFailure() {
  return useQuery({ queryKey: keys.startFailure, queryFn: () => commands.getStartFailure() }).data;
}

/** Interface language tag: the app state's, the saved one after a failed start, else `ru`. */
export function useLocale(): string {
  const state = useAppState();
  const failure = useStartFailure();
  return state?.locale ?? failure?.locale ?? "ru";
}

/** Version, compile time and git revision of this build; never changes while the app runs. */
export function useBuild() {
  return useQuery({ queryKey: keys.build, queryFn: () => commands.getBuild() }).data;
}
