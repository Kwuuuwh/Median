import type { Era, Fissure, Node } from "@median/ipc";
import { Bell, BellRing, Rocket, RotateCcw, X } from "lucide-react";
import { watchLabel } from "@/features/alarms/model/label";
import { useLocale } from "@/shared/data/app-state";
import { useWatches, useWatchToggle, watchKey } from "@/shared/data/watches";
import { useT } from "@/shared/i18n";
import { nameCollator } from "@/shared/lib/sort";
import { useKept } from "@/shared/lib/use-kept";
import { Button } from "@/shared/ui/button";
import { MultiSelect } from "@/shared/ui/multi-select";
import { Segmented } from "@/shared/ui/segmented";
import { TileGrid } from "@/shared/ui/tile-grid";
import { Toggle } from "@/shared/ui/toggle";
import {
  ERAS,
  type FissureFilter,
  type FissureScope,
  listFissures,
  NO_FILTER,
  narrows,
  watchesFor,
} from "../model/fissures";
import { MISSION_CODES } from "../model/missions";
import { useRemaining } from "./remaining";

/** Unique fissure key; a node can have a normal and a Steel Path fissure at once. */
function keyOf(fissure: Fissure): string {
  return `${fissure.node}:${fissure.hard}:${fissure.storm}`;
}

function FissureRow({
  fissure,
  node,
  now,
}: {
  fissure: Fissure;
  node: Node | undefined;
  now: number;
}) {
  const t = useT();
  const remaining = useRemaining();
  return (
    <li className="border-hairline border-t px-3 py-2 first:border-t-0">
      <div className="flex items-center gap-1.5 text-body text-ink">
        {fissure.hard && (
          <span className="shrink-0 rounded-sm bg-danger/15 px-1 font-semibold text-caption text-danger">
            {t("fissure-hard")}
          </span>
        )}
        {fissure.storm && (
          <Rocket aria-label={t("fissure-railjack")} className="size-3.5 shrink-0 text-muted">
            <title>{t("fissure-railjack")}</title>
          </Rocket>
        )}
        <span className="min-w-0">{node?.mission ?? fissure.node}</span>
      </div>
      <div className="flex items-baseline gap-2 text-small">
        <span className="min-w-0 flex-1 text-muted">
          {node ? `${node.name}, ${node.planet}` : ""}
        </span>
        <span className="shrink-0 text-soft tabular-nums">{remaining(fissure.expires - now)}</span>
      </div>
    </li>
  );
}

/** Active fissure watches as chips, each with a remove button; nothing when there are none. */
function FissureWatches() {
  const t = useT();
  const toggle = useWatchToggle();
  const watches = useWatches().filter((watch) => watch.kind === "fissure");
  if (watches.length === 0) return null;
  return (
    <ul className="flex flex-wrap gap-1.5">
      {watches.map((watch) => (
        <li
          key={watchKey(watch)}
          className="inline-flex h-7 items-center gap-1.5 rounded-full border border-line pr-1 pl-2.5 text-small text-soft"
        >
          <BellRing className="size-3.5 text-muted" />
          {watchLabel(t, watch)}
          <button
            type="button"
            aria-label={t("alarms-drop")}
            onClick={() => toggle.mutate({ watch, on: false })}
            className="inline-flex size-5 items-center justify-center rounded-full text-dim hover:bg-raised hover:text-ink"
          >
            <X className="size-3" />
          </button>
        </li>
      ))}
    </ul>
  );
}

/**
 * Open fissures in one column per era, filtered by era, scope and mission; the same filter can be
 * saved as a watch.
 */
export function FissuresPanel({
  fissures,
  nodes,
  now,
}: {
  fissures: readonly Fissure[];
  nodes: ReadonlyMap<string, Node>;
  now: number;
}) {
  const t = useT();
  const locale = useLocale();
  const [filter, setFilter] = useKept<FissureFilter>("fissures.filter", NO_FILTER);
  const toggle = useWatchToggle();
  const taken = new Set(useWatches().map(watchKey));
  const listed = listFissures(fissures, filter, now);
  const eras = filter.eras.size > 0 ? ERAS.filter((era) => filter.eras.has(era)) : ERAS;
  const collator = nameCollator(locale);
  const named = new Map(
    fissures.flatMap((fissure) => {
      const name = nodes.get(fissure.node)?.mission;
      return fissure.mission && name ? [[fissure.mission, name] as const] : [];
    }),
  );
  const missions = [
    ...new Set([
      ...MISSION_CODES,
      ...fissures.flatMap((fissure) => (fissure.mission ? [fissure.mission] : [])),
    ]),
  ]
    .map((code) => ({
      value: code,
      label: named.get(code) ?? t("mission-type", { mission: code }),
    }))
    .sort((a, b) => collator.compare(a.label, b.label));
  const wanted = watchesFor(filter);
  const allTaken = wanted.every((watch) => taken.has(watchKey(watch)));

  function flipEra(era: Era): void {
    const picked = new Set(filter.eras);
    if (picked.has(era)) picked.delete(era);
    else picked.add(era);
    setFilter({ ...filter, eras: picked });
  }

  return (
    <section className="grid gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <div className="flex gap-1">
          {ERAS.map((era) => (
            <Toggle key={era} pressed={filter.eras.has(era)} onPressedChange={() => flipEra(era)}>
              {t("era-name", { era })}
            </Toggle>
          ))}
        </div>
        <Segmented<FissureScope>
          label={t("timers-fissures")}
          className="h-7"
          value={filter.scope}
          onChange={(scope) => setFilter({ ...filter, scope })}
          segments={[
            { value: "all", label: t("fissure-all") },
            { value: "plain", label: t("fissure-plain") },
            { value: "hard", label: t("fissure-hard-long") },
          ]}
        />
        <MultiSelect
          label={t("watch-any-mission")}
          className="h-7 w-48"
          values={filter.missions}
          choices={missions}
          onChange={(picked) => setFilter({ ...filter, missions: picked })}
        />
        <Toggle
          pressed={filter.railjack}
          onPressedChange={(railjack) => setFilter({ ...filter, railjack })}
        >
          <Rocket />
          {t("fissure-railjack")}
        </Toggle>
        <Button
          size="sm"
          tone="ghost"
          disabled={!narrows(filter) && !filter.railjack}
          onClick={() => setFilter(NO_FILTER)}
        >
          <RotateCcw />
          {t("fissure-reset")}
        </Button>
        <Button
          size="sm"
          disabled={!narrows(filter) || allTaken}
          onClick={() => {
            for (const watch of wanted) toggle.mutate({ watch, on: true });
          }}
        >
          <Bell />
          {t("watch-take")}
        </Button>
      </div>
      <FissureWatches />
      <TileGrid least={208}>
        {eras.map((era) => {
          const column = listed.filter((fissure) => fissure.era === era);
          return (
            <div
              key={era}
              className="min-w-0 overflow-hidden rounded-panel border border-line bg-surface"
            >
              <div className="flex items-baseline justify-between border-hairline border-b px-3 py-2">
                <span className="font-medium text-body text-ink">{t("era-name", { era })}</span>
                <span className="text-muted text-small tabular-nums">{column.length}</span>
              </div>
              {column.length > 0 && (
                <ul>
                  {column.map((fissure) => (
                    <FissureRow
                      key={keyOf(fissure)}
                      fissure={fissure}
                      node={nodes.get(fissure.node)}
                      now={now}
                    />
                  ))}
                </ul>
              )}
            </div>
          );
        })}
      </TileGrid>
    </section>
  );
}
