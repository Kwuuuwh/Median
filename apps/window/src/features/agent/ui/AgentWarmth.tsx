import type { Agent } from "@median/ipc";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { warmthOf } from "../model/warmth";

/**
 * Progress bar of watched items fetched from the market since app start, as `fetched/watched`;
 * an ellipsis while the watched items are being counted.
 */
export function AgentWarmth({ agent }: { agent: Pick<Agent, "warm" | "hot"> }) {
  const t = useT();
  const warmth = warmthOf(agent);
  const warm = warmth.phase === "warm";
  return (
    <div className="grid w-28 shrink-0 gap-1.5">
      <span className="flex items-baseline gap-1 text-body">
        <span className="text-muted">{t("agent-warmth")}</span>
        {warmth.phase === "counting" ? (
          <span className="text-dim">…</span>
        ) : (
          <span
            className={cn(
              "tabular-nums transition-colors duration-700",
              warm ? "text-live" : "text-ink",
            )}
          >
            {warmth.warm}
            <span className="text-dim">/</span>
            {warmth.hot}
          </span>
        )}
      </span>
      <span
        className={cn(
          "relative h-1 overflow-hidden rounded-full bg-line",
          warm && "animate-warmed",
        )}
      >
        {warmth.phase !== "counting" && (
          <span
            className={cn(
              "absolute inset-y-0 left-0 rounded-full transition-[width] duration-500 ease-out",
              warm && "bg-live/60",
            )}
            style={{
              width: `${warmth.share * 100}%`,
              ...(!warm && {
                backgroundImage: "linear-gradient(90deg, var(--paint-accent), var(--paint-warn))",
                backgroundSize: `${100 / Math.max(warmth.share, 0.01)}% 100%`,
              }),
            }}
          />
        )}
        {!warm && (
          <span className="absolute inset-y-0 w-1/3 animate-glint bg-gradient-to-r from-transparent via-ink/35 to-transparent motion-reduce:hidden" />
        )}
      </span>
    </div>
  );
}
