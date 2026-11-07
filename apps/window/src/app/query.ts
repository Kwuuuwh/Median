import { DEFAULT_LOCALE, translate } from "@median/i18n";
import { type AppState, CommandError } from "@median/ipc";
import { MutationCache, QueryClient } from "@tanstack/react-query";
import { keys } from "@/shared/data/keys";
import { showNotice } from "@/shared/ui/toast";

/**
 * Query client for all command responses: queries never go stale on a timer because backend events
 * update them; a failed mutation shows a failure toast.
 */
export const queryClient: QueryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: Number.POSITIVE_INFINITY, retry: false, refetchOnWindowFocus: false },
  },
  mutationCache: new MutationCache({
    onError: (error) => {
      const locale = queryClient.getQueryData<AppState>(keys.state)?.locale ?? DEFAULT_LOCALE;
      showNotice({
        tone: "failure",
        title: translate(locale, "toast-command-failed"),
        detail: error instanceof CommandError ? String(error.reason) : String(error),
      });
    },
  }),
});
