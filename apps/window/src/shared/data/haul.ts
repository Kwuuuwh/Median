import { commands, type Haul, unwrap } from "@median/ipc";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocale } from "./app-state";
import { keys } from "./keys";

/**
 * Refetch interval of the haul, because the account history stores an inventory reading shortly
 * after the window receives the inventory event.
 */
const HAUL_REFRESH_MS = 30_000;

/** Items the account gained since the haul started, in the interface language. */
export function useHaul() {
  const lang = useLocale();
  return useQuery<Haul>({
    queryKey: keys.haul(lang),
    queryFn: () => unwrap(commands.getHaul(lang)),
    refetchInterval: HAUL_REFRESH_MS,
  });
}

/** Restarts the haul from the current inventory. */
export function useRestartHaul() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: () => unwrap(commands.restartHaul()),
    onSettled: () => void client.invalidateQueries({ queryKey: keys.haulAll }),
  });
}
