import { commands, type MasteryRank, type Planned, type Roadmap, unwrap } from "@median/ipc";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocale } from "./app-state";
import { keys } from "./keys";

/** The account's mastery rank and progress, and every item that gives mastery. */
export function useMastery() {
  const lang = useLocale();
  return useQuery<MasteryRank>({
    queryKey: keys.mastery(lang),
    queryFn: () => unwrap(commands.getMastery(lang)),
  });
}

/** The mastery plan: its items and their combined requirements. */
export function useRoadmap() {
  const lang = useLocale();
  return useQuery<Roadmap>({
    queryKey: keys.roadmap(lang),
    queryFn: () => unwrap(commands.getRoadmap(lang)),
  });
}

/** A plan mutation that refetches the plan once it settles. */
function useRoadmapChange<T>(change: (input: T) => Promise<unknown>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: change,
    onSettled: () => void client.invalidateQueries({ queryKey: keys.roadmapAll }),
  });
}

/** Adds or removes a plan item; the cache updates at once, then the plan is refetched. */
export function usePlanToggle() {
  const client = useQueryClient();
  const lang = useLocale();
  return useMutation({
    mutationFn: ({ item, on }: { item: string; on: boolean }) =>
      unwrap(commands.setRoadmap(item, on)),
    onMutate: async ({ item, on }) => {
      await client.cancelQueries({ queryKey: keys.roadmap(lang) });
      client.setQueryData<Roadmap>(keys.roadmap(lang), (plan) => {
        if (!plan) return plan;
        const others = plan.planned.filter((planned) => planned.item !== item);
        if (!on) return { ...plan, planned: others };
        const added: Planned = {
          item,
          name: item,
          image: null,
          owned: false,
          remaining: 0,
        };
        return { ...plan, planned: [...others, added] };
      });
    },
    onSettled: () => void client.invalidateQueries({ queryKey: keys.roadmapAll }),
  });
}

/** Removes every item from the plan. */
export function useClearPlan() {
  return useRoadmapChange(() => unwrap(commands.clearRoadmap()));
}
