import { type AppState, commands } from "@median/ipc";
import { useQuery } from "@tanstack/react-query";
import { keys } from "./keys";

/** App state from the backend, kept current by its events; undefined until the first response. */
export function useAppState(): AppState | undefined {
  return useQuery({ queryKey: keys.state, queryFn: () => commands.getState() }).data;
}

/** Interface language tag; `ru` until the app state loads. */
export function useLocale(): string {
  return useAppState()?.locale ?? "ru";
}

/** Version, compile time and git revision of this build; never changes while the app runs. */
export function useBuild() {
  return useQuery({ queryKey: keys.build, queryFn: () => commands.getBuild() }).data;
}
