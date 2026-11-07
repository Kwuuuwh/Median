import type { Cycle, Deadline, Node, Vendor, Visit } from "@median/ipc";
import type { ReactNode } from "react";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { currentPhase } from "../model/phase";
import { useRemaining } from "./remaining";
import { WatchBell, WatchMenu } from "./WatchBell";

/** Time left in seconds under which a timer is highlighted as ending soon. */
const SOON_SECS = 15 * 60;

/** What the right side shows: fissures, syndicates, or a vendor's offers. */
export type Detail = "fissures" | "syndicates" | Vendor;

function phaseColor(phase: string): string {
  return `var(--paint-phase-${phase}, var(--paint-muted))`;
}

/** One timer row: name, current state, time left and a bell; `onSelect` makes it selectable. */
function NowRow({
  name,
  line,
  secs,
  bell,
  selected,
  onSelect,
}: {
  name: string;
  line?: ReactNode;
  secs: number;
  bell?: ReactNode;
  selected?: boolean;
  onSelect?: () => void;
}) {
  const remaining = useRemaining();
  const body = (
    <>
      <div className="min-w-0 flex-1">
        <div className="truncate text-body text-ink">{name}</div>
        {line && <div className="text-muted text-small first-letter:uppercase">{line}</div>}
      </div>
      <span
        className={cn(
          "shrink-0 font-medium text-body tabular-nums",
          secs < SOON_SECS ? "text-warn" : "text-soft",
        )}
      >
        {remaining(secs)}
      </span>
      <span className="flex w-6 shrink-0 justify-center">{bell}</span>
    </>
  );
  const frame = "flex w-full items-center gap-2 rounded-control px-2.5 py-1.5 text-left";
  if (!onSelect) return <div className={frame}>{body}</div>;
  return (
    // biome-ignore lint/a11y/useSemanticElements: a button cannot contain the row's bell button
    <div
      role="button"
      tabIndex={0}
      aria-pressed={selected}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onSelect();
        }
      }}
      className={cn(
        frame,
        "cursor-default transition-colors hover:bg-raised",
        selected && "bg-raised",
      )}
    >
      {body}
    </div>
  );
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h2 className="mb-1 px-2.5 font-medium text-caption text-dim">{title}</h2>
      <div className="grid gap-0.5">{children}</div>
    </section>
  );
}

/** Column of every world timer as rows: cycles, resets and vendor visits. */
export function NowColumn({
  cycles,
  deadlines,
  visits,
  nodes,
  now,
  detail,
  onDetail,
}: {
  cycles: readonly Cycle[];
  deadlines: readonly Deadline[];
  visits: readonly Visit[];
  nodes: ReadonlyMap<string, Node>;
  now: number;
  detail: Detail;
  onDetail: (detail: Detail) => void;
}) {
  const t = useT();
  const order: Record<string, number> = { daily: 0, sortie: 1, weekly: 2 };

  return (
    <div className="grid gap-4">
      <Group title={t("timers-cycles")}>
        {cycles.map((cycle) => {
          const at = currentPhase(cycle, now);
          if (!at) return null;
          return (
            <NowRow
              key={cycle.key}
              name={t("cycle-name", { cycle: cycle.key })}
              line={
                <span className="inline-flex items-center gap-1.5">
                  <span
                    className="size-1.5 rounded-full"
                    style={{ background: phaseColor(at.phase) }}
                  />
                  <span className="first-letter:uppercase" style={{ color: phaseColor(at.phase) }}>
                    {t("cycle-phase", { phase: at.phase })}
                  </span>
                  <span className="text-dim">→ {t("cycle-phase", { phase: at.next })}</span>
                </span>
              }
              secs={at.ends - now}
              bell={
                <WatchMenu
                  label={t("watch-phase")}
                  choices={[...new Set(cycle.phases.map((phase) => phase.key))].map((phase) => ({
                    watch: { kind: "phase", cycle: cycle.key, phase },
                    label: t("cycle-phase", { phase }),
                  }))}
                />
              }
            />
          );
        })}
      </Group>
      <Group title={t("timers-resets")}>
        {[...deadlines]
          .sort((a, b) => order[a.reset] - order[b.reset])
          .map((deadline) => (
            <NowRow
              key={deadline.reset}
              name={t("reset-name", { reset: deadline.reset })}
              secs={deadline.at - now}
            />
          ))}
      </Group>
      <Group title={t("timers-rotations")}>
        {visits.map((visit) => {
          const here = visit.arrives <= now && now < visit.departs;
          const node = nodes.get(visit.node);
          const line =
            visit.vendor === "varzia"
              ? t("trader-rotation")
              : `${t(here ? "trader-here" : "trader-road")} · ${node ? `${node.name}, ${node.planet}` : visit.node}`;
          return (
            <NowRow
              key={visit.vendor}
              name={t("trader-name", { vendor: visit.vendor })}
              line={line}
              secs={(here || visit.vendor === "varzia" ? visit.departs : visit.arrives) - now}
              bell={
                visit.vendor === "baro" ? (
                  <WatchBell
                    watch={{ kind: "arrival", vendor: "baro" }}
                    label={t("watch-arrival")}
                  />
                ) : undefined
              }
              selected={detail === visit.vendor}
              onSelect={() => onDetail(visit.vendor)}
            />
          );
        })}
      </Group>
    </div>
  );
}
