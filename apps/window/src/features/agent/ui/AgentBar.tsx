import type { OwnOrder } from "@median/ipc";
import { Power, SlidersHorizontal } from "lucide-react";
import type { ReactNode } from "react";
import { useAppState } from "@/shared/data/app-state";
import { useFigures, useSetAgent } from "@/shared/data/trading";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Button } from "@/shared/ui/button";
import { Money } from "@/shared/ui/money";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";
import { Toggle } from "@/shared/ui/toggle";
import { agentBids, blockersOf, budgetOf, tiedBy } from "../model/agent";
import { byAgent, summarize } from "../model/stats";
import { AgentPulse } from "./AgentPulse";
import { AgentSettings } from "./AgentSettings";
import { AgentWarmth } from "./AgentWarmth";

/** One value of the bar with its label above it. */
function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid gap-0.5">
      <span className="text-caption text-dim">{label}</span>
      <span className="text-body text-ink tabular-nums">{children}</span>
    </div>
  );
}

/**
 * Agent summary bar: on/off toggle, buy order count, platinum committed against the budget, today's
 * margin, exchanges left, fetch progress, activity, blockers, and a settings popover.
 */
export function AgentBar({ orders }: { orders: readonly OwnOrder[] }) {
  const t = useT();
  const state = useAppState();
  const setAgent = useSetAgent();
  const today = useFigures(1).data;
  if (!state) return null;

  const bids = agentBids(orders);
  const tied = tiedBy(bids);
  const budget = budgetOf(state.policy, state.account.platinum);
  const made = summarize(byAgent(today?.deals ?? [])).margin;
  const running = state.agent.running;
  const blockers = blockersOf(state, budget);

  return (
    <div className="flex shrink-0 items-center gap-6 border-line border-b px-6 py-2.5">
      <Toggle
        pressed={running}
        onPressedChange={(on) => setAgent.mutate(on)}
        aria-label={t("agent-name")}
        className="h-8 px-3"
      >
        <Power />
        {t("agent-name")}
      </Toggle>
      <Fact label={t("agent-bids")}>{bids.length}</Fact>
      <Fact label={t("agent-tied")}>
        <span className="inline-flex items-center gap-1">
          <Money currency="platinum" amount={tied} />
          <span className="text-dim">/</span>
          <Money currency="platinum" amount={budget} className="text-soft" />
        </span>
      </Fact>
      <Fact label={t("agent-today")}>
        <span className={cn(made > 0 && "text-live", made < 0 && "text-danger")}>
          <Money currency="platinum" amount={made} />
        </span>
      </Fact>
      <Fact label={t("agent-exchanges")}>{state.account.exchanges ?? "—"}</Fact>
      <AgentWarmth agent={state.agent} />
      <AgentPulse agent={state.agent} blockers={blockers} />
      {blockers.length > 0 && (
        <span className="flex flex-wrap gap-1.5">
          {blockers.map((blocker) => (
            <span
              key={blocker}
              className={cn(
                "rounded-full px-2 py-0.5 text-small",
                running ? "bg-warn/15 text-warn" : "bg-raised text-muted",
              )}
            >
              {t("agent-blocker", { blocker })}
            </span>
          ))}
        </span>
      )}
      <Popover>
        <PopoverTrigger asChild>
          <Button tone="ghost" size="icon" aria-label={t("agent-settings")} className="ml-auto">
            <SlidersHorizontal />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end">
          <AgentSettings policy={state.policy} />
        </PopoverContent>
      </Popover>
    </div>
  );
}
