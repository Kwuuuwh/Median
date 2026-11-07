import { commands, type Syndicate, unwrap } from "@median/ipc";
import { useQuery } from "@tanstack/react-query";
import { useLocale } from "./app-state";
import { keys } from "./keys";

/** Each syndicate with the account's standing and the vendor offers it buys, best first. */
export function useSyndicates() {
  const lang = useLocale();
  return useQuery<Syndicate[]>({
    queryKey: keys.syndicates(lang),
    queryFn: () => unwrap(commands.getSyndicates(lang)),
  });
}
