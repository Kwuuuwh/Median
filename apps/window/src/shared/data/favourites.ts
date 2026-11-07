import { commands, unwrap } from "@median/ipc";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { keys } from "./keys";

/** Item keys marked as favourites. */
export function useFavourites() {
  return useQuery<Set<string>>({
    queryKey: keys.favourites,
    queryFn: async () => new Set(await unwrap(commands.getFavourites())),
  });
}

/** Sets or clears an item's favourite mark; the cache updates at once and rolls back on error. */
export function useFavouriteToggle() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ item, on }: { item: string; on: boolean }) =>
      unwrap(commands.setFavourite(item, on)),
    onMutate: async ({ item, on }) => {
      await client.cancelQueries({ queryKey: keys.favourites });
      const before = client.getQueryData<Set<string>>(keys.favourites);
      const after = new Set(before);
      if (on) after.add(item);
      else after.delete(item);
      client.setQueryData(keys.favourites, after);
      return { before };
    },
    onError: (_error, _toggle, context) => {
      client.setQueryData(keys.favourites, context?.before);
    },
  });
}
