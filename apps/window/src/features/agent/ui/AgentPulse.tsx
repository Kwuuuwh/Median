import type { Agent, Seen } from "@median/ipc";
import { useEffect, useState } from "react";
import { openItem } from "@/features/items/model/open";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { nowSecs, useNow } from "@/shared/lib/use-now";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Money } from "@/shared/ui/money";
import { Tooltip } from "@/shared/ui/tooltip";
import type { Blocker } from "../model/agent";
import { type Count, type Pulse, paceOf, pulseOf } from "../model/pulse";

/** Samples of the running count kept for measuring the pace. */
const COUNTS = 120;

/** Evaluated items the strip shows, newest first. */
const SHOWN = 14;

/** Dot colour of each activity state. */
const DOT: Record<Pulse, string> = {
  off: "bg-dim",
  blocked: "bg-warn",
  looking: "bg-live",
  waiting: "bg-accent",
};

/**
 * One item the agent evaluated: highlighted when accepted, faded when rejected; the tooltip shows
 * entry price, value, and the outcome or rejection reason.
 */
function Look({ seen }: { seen: Seen }) {
  const t = useT();
  const took = seen.passed === null;
  return (
    <Tooltip
      hint={
        <span className="grid gap-1">
          <span className="text-ink">{seen.name}</span>
          <span className="flex items-center gap-2 text-soft">
            <Money currency="platinum" amount={seen.entry} />
            <span className="text-dim">→</span>
            <Money currency="platinum" amount={seen.worth} />
          </span>
          <span className={took ? "text-live" : "text-muted"}>
            {took ? t("agent-took") : t("agent-reason", { reason: seen.passed ?? "" })}
          </span>
        </span>
      }
    >
      <button
        type="button"
        disabled={!seen.item}
        onClick={() => openItem(seen.item)}
        className={cn(
          "fade-in-0 slide-in-from-left-3 relative size-9 shrink-0 animate-in rounded-control border bg-sunken transition-opacity duration-500",
          took ? "border-live/70" : "border-line opacity-45 hover:opacity-100",
        )}
      >
        <ItemPicture image={seen.image} className="size-full p-0.5" />
      </button>
    </Tooltip>
  );
}

/** Agent activity: a status dot that pulses while evaluating, the state and items per minute, and
 * the most recently evaluated items, newest on the left. */
export function AgentPulse({ agent, blockers }: { agent: Agent; blockers: readonly Blocker[] }) {
  const t = useT();
  const now = useNow();
  const pulse = pulseOf(agent, blockers, now);
  const [counts, setCounts] = useState<Count[]>([]);
  useEffect(() => {
    setCounts((before) => [...before, { at: nowSecs(), seen: agent.seen }].slice(-COUNTS));
  }, [agent.seen]);
  const pace = paceOf(counts, now);
  const looks = [...agent.flow].sort((a, b) => b.at - a.at).slice(0, SHOWN);
  return (
    <div className="flex min-w-0 flex-1 items-center gap-4">
      <span className="flex shrink-0 items-center gap-2.5">
        <span className="relative flex size-2.5">
          {pulse === "looking" && (
            <span className="absolute inset-0 animate-ping rounded-full bg-live opacity-60" />
          )}
          <span className={cn("relative size-2.5 rounded-full", DOT[pulse])} />
        </span>
        <span className="grid leading-tight">
          <span className="text-body text-ink">{t("agent-pulse", { pulse })}</span>
          {pulse === "looking" && pace !== null && (
            <span className="text-muted text-small tabular-nums">
              {t("agent-pace", { count: pace })}
            </span>
          )}
        </span>
      </span>
      {agent.running && looks.length > 0 && (
        <span className="flex min-w-0 gap-1.5 overflow-hidden [mask-image:linear-gradient(to_right,black_80%,transparent)]">
          {looks.map((seen) => (
            <Look key={`${seen.at}:${seen.item}:${seen.entry}`} seen={seen} />
          ))}
        </span>
      )}
    </div>
  );
}
