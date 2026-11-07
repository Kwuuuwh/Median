import {
  type Amend,
  commands,
  type Figures,
  type OwnOrder,
  type Policy,
  type Position,
  unwrap,
} from "@median/ipc";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocale } from "./app-state";
import { keys } from "./keys";

/** Agent positions (items bought for resale), each with its current market prices. */
export function usePositions() {
  const lang = useLocale();
  return useQuery<Position[]>({
    queryKey: keys.positions(lang),
    queryFn: () => unwrap(commands.getPositions(lang)),
  });
}

/** Totals of the exchanges in the last `days`; `0` covers all exchanges. */
export function useFigures(days: number) {
  const lang = useLocale();
  return useQuery<Figures>({
    queryKey: keys.figures(lang, days),
    queryFn: () => unwrap(commands.getFigures(lang, days)),
  });
}

/** Starts or stops the agent; the new state arrives with the backend's event. */
export function useSetAgent() {
  return useMutation({ mutationFn: (on: boolean) => unwrap(commands.setAgent(on)) });
}

/** Sets the agent's policy limits; the new state arrives with the backend's event. */
export function useSetPolicy() {
  return useMutation({ mutationFn: (policy: Policy) => unwrap(commands.setPolicy(policy)) });
}

/**
 * A mutation of the account's orders that refetches orders and positions once it settles; an error
 * shows as a toast.
 */
function useOrderChange<V, R>(run: (vars: V) => Promise<R>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSettled: () => {
      void client.invalidateQueries({ queryKey: keys.ordersAll });
      void client.invalidateQueries({ queryKey: keys.positionsAll });
    },
  });
}

/** Changes one order's price, quantity or visibility; the cached list updates at once. */
export function useAmendOrder() {
  const lang = useLocale();
  const client = useQueryClient();
  const change = useOrderChange(({ id, amend }: { id: string; amend: Amend }) =>
    unwrap(commands.amendOrder(id, amend, lang)),
  );
  return {
    ...change,
    mutate: (vars: { id: string; amend: Amend }) => {
      client.setQueryData<OwnOrder[]>(keys.orders(lang), (orders) =>
        orders?.map((order) =>
          order.id === vars.id
            ? {
                ...order,
                platinum: vars.amend.platinum ?? order.platinum,
                quantity: vars.amend.quantity ?? order.quantity,
                visible: vars.amend.visible ?? order.visible,
              }
            : order,
        ),
      );
      change.mutate(vars);
    },
  };
}

/** Deletes orders; they leave the cached list at once. */
export function useWithdrawOrders() {
  const lang = useLocale();
  const client = useQueryClient();
  const change = useOrderChange((ids: string[]) => unwrap(commands.withdrawOrders(ids)));
  return {
    ...change,
    mutate: (ids: string[]) => {
      const gone = new Set(ids);
      client.setQueryData<OwnOrder[]>(keys.orders(lang), (orders) =>
        orders?.filter((order) => !gone.has(order.id)),
      );
      change.mutate(ids);
    },
  };
}

/** Makes every buy or every sell order visible or hidden. */
export function useShowOrders() {
  return useOrderChange(({ selling, visible }: { selling: boolean; visible: boolean }) =>
    unwrap(commands.showOrders(selling, visible)),
  );
}

/** Fetches the account's orders from warframe.market again. */
export function useRefreshOrders() {
  return useOrderChange(() => unwrap(commands.refreshOrders()));
}

/** Passes copies the user bought to the agent to sell; `platinum` is the cost of one copy. */
export function useHandToAgent() {
  return useOrderChange(
    ({
      item,
      variant,
      quantity,
      platinum,
    }: {
      item: string;
      variant: string;
      quantity: number;
      platinum: number;
    }) => unwrap(commands.handToAgent(item, variant, platinum, quantity)),
  );
}

/** Passes one of the user's orders to the agent; for a sell order `platinum` is one copy's cost. */
export function useHandOrderToAgent() {
  return useOrderChange(({ id, platinum }: { id: string; platinum: number | null }) =>
    unwrap(commands.handOrderToAgent(id, platinum)),
  );
}

/** Takes a position away from the agent; its order stays open as the user's own. */
export function useReleasePosition() {
  return useOrderChange(({ slug, variant }: { slug: string; variant: string }) =>
    unwrap(commands.releasePosition(slug, variant)),
  );
}

/** Records a completed exchange and closes the order it filled. */
export function useRecordExchange() {
  return useOrderChange(
    ({ order, quantity, platinum }: { order: OwnOrder; quantity: number; platinum: number }) =>
      unwrap(
        commands.recordExchange(
          order.id,
          order.item,
          order.slug,
          order.variant,
          order.selling,
          platinum,
          quantity,
        ),
      ),
  );
}
