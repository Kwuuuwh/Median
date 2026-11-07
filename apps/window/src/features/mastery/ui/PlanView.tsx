import type { MasteryRank, Requirement, Roadmap } from "@median/ipc";
import { type ReactNode, useEffect, useState } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useClearPlan } from "@/shared/data/mastery";
import { useItemSearch } from "@/shared/data/search";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount, formatSpan } from "@/shared/lib/format";
import { useKept } from "@/shared/lib/use-kept";
import { Button } from "@/shared/ui/button";
import { EmptyState } from "@/shared/ui/empty-state";
import { Money } from "@/shared/ui/money";
import { SearchField } from "@/shared/ui/search-field";
import { Segmented } from "@/shared/ui/segmented";
import { VirtualGrid } from "@/shared/ui/virtual-grid";
import { byGet, levelling, NEEDS, type Need, needCounts, needLines, shortOf } from "../model/plan";
import { PLAN_TILE_HEIGHT, PLAN_TILE_WIDTH, PlanTile } from "./PlanTile";

/** Area choice that shows every area. */
const EVERYWHERE = "everywhere";

/** How long the clear button waits for the confirming second click, in milliseconds. */
const ARMED_MS = 3_000;

/** A plan step's name with the number of lines that still need work. */
function NeedLabel({ name, count }: { name: string; count: number }) {
  return (
    <>
      {name}
      <span className="text-muted tabular-nums">{count}</span>
    </>
  );
}

/** One plan total: label and value. */
function Figure({ label, children }: { label: string; children: ReactNode }) {
  return (
    <span className="grid gap-0.5">
      <span className="text-muted text-small">{label}</span>
      <span className="font-medium text-body text-ink tabular-nums">{children}</span>
    </span>
  );
}

/** One tile of a plan step. */
type Tile = {
  item: string;
  name: string;
  image: string | null;
  have: number;
  need: number;
  startable: boolean;
};

/** Tiles under one heading; `title` is null when a step shows all its tiles in one group. */
type Group = { title: string | null; tiles: Tile[] };

/** The mastery plan: its gain and cost totals, then per step the remaining work as tiles. */
export function PlanView({ standing, roadmap }: { standing: MasteryRank; roadmap: Roadmap }) {
  const t = useT();
  const locale = useLocale();
  const clear = useClearPlan();
  const [keptArea, setArea] = useKept("plan.area", EVERYWHERE);
  const [keptNeed, setNeed] = useKept<Need>("plan.need", "forge");
  const [armed, setArmed] = useState(false);
  const [scroller, setScroller] = useState<HTMLDivElement | null>(null);
  const { total } = roadmap;
  const area = roadmap.areas.some((each) => each.key === keptArea) ? keptArea : null;
  const startable = new Set(roadmap.ready.map((each) => each.item));
  const every = [...total.lines.filter((line) => !line.built), ...roadmap.forge];
  const search = useItemSearch("plan.search", [
    ...every.map((line) => line.item),
    ...roadmap.planned.map((each) => each.item),
  ]);
  const lines = every.filter((line) => search.matches(line.item, line.name));
  const levelled = levelling(roadmap.planned, standing.masterable).filter(({ planned }) =>
    search.matches(planned.item, planned.name),
  );
  const counts = needCounts(lines, area, area === null ? levelled.length : 0);
  const needs = NEEDS.filter((each) => (counts.get(each) ?? 0) > 0);
  const need = needs.includes(keptNeed) ? keptNeed : (needs[0] ?? keptNeed);

  const tileOf = (line: Requirement): Tile => ({
    item: line.item,
    name: line.name,
    image: line.image,
    have: line.have + line.forging,
    need: line.count,
    startable: need === "forge" && (startable.has(line.item) || shortOf(line) === 0),
  });
  const groups: Group[] =
    need === "level"
      ? [
          {
            title: null,
            tiles: levelled.map(({ planned, rank, cap }) => ({
              item: planned.item,
              name: planned.name,
              image: planned.image,
              have: rank,
              need: cap,
              startable: false,
            })),
          },
        ]
      : need === "obtain"
        ? byGet(needLines(lines, need, area, startable)).map((group) => ({
            title: t("plan-get", { get: group.get }),
            tiles: group.lines.map(tileOf),
          }))
        : [{ title: null, tiles: needLines(lines, need, area, startable).map(tileOf) }];

  useEffect(() => {
    if (!armed) return;
    const timer = setTimeout(() => setArmed(false), ARMED_MS);
    return () => clearTimeout(timer);
  }, [armed]);

  if (roadmap.planned.length === 0)
    return (
      <div className="flex min-h-0 flex-1 flex-col items-center justify-center">
        <EmptyState title={t("plan-empty")} />
      </div>
    );

  return (
    <div ref={setScroller} className="relative min-h-0 flex-1 overflow-y-auto px-6 pt-3 pb-16">
      <div className="flex flex-wrap items-end gap-x-8 gap-y-3">
        <Figure label={t("roadmap-sum-gain")}>
          <span className="text-live">
            {t("roadmap-gain", { gain: formatCount(locale, roadmap.gain) })}
          </span>
          {roadmap.reaches !== null && roadmap.reaches > (standing.rank ?? 0) && (
            <span className="ml-2 text-muted text-small">
              {t("roadmap-gain-rank", { rank: roadmap.reaches })}
            </span>
          )}
        </Figure>
        <Figure label={t("roadmap-sum-credits")}>
          <Money
            currency="credits"
            amount={total.credits}
            className={cn(total.purse !== null && total.purse < total.credits && "text-danger")}
          />
        </Figure>
        <Figure label={t("roadmap-sum-time")}>{formatSpan(t, total.time)}</Figure>
        <span className="flex-1" />
        <Button
          tone="danger"
          onClick={() => {
            if (!armed) return setArmed(true);
            setArmed(false);
            clear.mutate(undefined);
          }}
        >
          {t(armed ? "roadmap-clear-confirm" : "roadmap-clear")}
        </Button>
      </div>

      <div className="mt-4 flex flex-wrap items-center gap-2">
        <Segmented<Need>
          label={t("plan-needs")}
          value={need}
          onChange={setNeed}
          segments={needs.map((each) => ({
            value: each,
            label: (
              <NeedLabel name={t("plan-need", { need: each })} count={counts.get(each) ?? 0} />
            ),
          }))}
        />
        {roadmap.areas.length > 0 && (
          <Segmented
            label={t("chain-everywhere")}
            value={area ?? EVERYWHERE}
            onChange={setArea}
            segments={[
              { value: EVERYWHERE, label: t("chain-everywhere") },
              ...roadmap.areas.map((each) => ({ value: each.key, label: each.name })),
            ]}
          />
        )}
        <span className="flex-1" />
        <SearchField
          {...search.field}
          placeholder={t("mastery-search")}
          className="w-56 min-w-36 shrink"
        />
      </div>

      <div className="mt-3">
        {groups.every((group) => group.tiles.length === 0) ? (
          <EmptyState title={t(search.field.value.trim() ? "mastery-none" : "plan-done")} />
        ) : (
          groups.map((group) => (
            <section key={group.title ?? need} className="mb-5">
              {group.title && (
                <h2 className="mb-2 flex items-baseline gap-2 font-medium text-body text-soft">
                  {group.title}
                  <span className="text-muted text-small tabular-nums">{group.tiles.length}</span>
                </h2>
              )}
              <VirtualGrid
                items={group.tiles}
                scroller={scroller}
                least={PLAN_TILE_WIDTH}
                estimate={PLAN_TILE_HEIGHT}
                keyOf={(tile) => tile.item}
              >
                {(tile) => <PlanTile {...tile} />}
              </VirtualGrid>
            </section>
          ))
        )}
      </div>
    </div>
  );
}
