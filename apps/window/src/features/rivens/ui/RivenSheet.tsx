import type { Good, Riven, RivenAppraisal, RivenChoices } from "@median/ipc";
import type { ReactNode } from "react";
import { useRivenBands, useRivenChoices, useRivenRolls } from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Skeleton } from "@/shared/ui/skeleton";
import { TileGrid } from "@/shared/ui/tile-grid";
import { AnalogCard } from "./AnalogCard";
import { Decisions } from "./Decisions";
import { RivenHeading } from "./RivenHeading";
import { RollTable } from "./RollTable";
import { Section } from "./Section";
import { StatRow } from "./StatRow";

/**
 * One riven's sheet: its weapon, stats and option values next to the reroll table, then similar
 * auctions on the market. Each part shows a placeholder until its data loads.
 */
export function RivenSheet({
  riven,
  rank,
  weapon,
  name,
  appraisal,
  trade,
}: {
  riven: Riven;
  rank: number;
  weapon: Good | undefined;
  name: string | null | undefined;
  appraisal: RivenAppraisal | null | undefined;
  /** Market actions for the riven, rendered under the option values. */
  trade?: (choices: RivenChoices | null | undefined) => ReactNode;
}) {
  const t = useT();
  const market = useRivenChoices(riven).data;
  const [kept] = useRivenBands([riven]);
  const rolls = useRivenRolls(riven).data;
  const isUnknown = market === null;
  const stats = appraisal?.stats ?? [];
  const holds = new Set([
    ...riven.buffs.map((stat) => `${stat.tag}false`),
    ...riven.curses.map((stat) => `${stat.tag}true`),
  ]);
  return (
    <div className="flex flex-col gap-5">
      <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] items-start gap-6">
        <div className="flex flex-col gap-4">
          <div className="grid grid-cols-[6.5rem_minmax(0,1fr)] gap-4">
            <div className="grid aspect-square place-items-center rounded-panel bg-sunken p-2">
              <ItemPicture image={weapon?.image ?? null} className="size-full" />
            </div>
            <div className="flex min-w-0 flex-col gap-2.5">
              <RivenHeading
                riven={riven}
                rank={rank}
                weapon={weapon}
                name={name}
                appraisal={appraisal}
              />
              <div className="flex flex-col gap-1">
                {appraisal === undefined
                  ? [...riven.buffs, ...riven.curses].map((stat) => (
                      <Skeleton key={stat.tag} className="h-5" />
                    ))
                  : stats.map((stat) => <StatRow key={`${stat.tag}${stat.isCurse}`} stat={stat} />)}
              </div>
            </div>
          </div>
          {!isUnknown && (
            <Decisions
              sell={market === undefined ? (kept ?? undefined) : market.sell}
              rolls={rolls}
              transmute={market === undefined ? undefined : market.transmute}
            />
          )}
          {trade?.(market)}
        </div>
        {!isUnknown && (rolls === undefined || (rolls && rolls.length > 0)) && (
          <Section title={t("riven-rolls")}>
            {rolls ? <RollTable odds={rolls} stats={stats} /> : <Skeleton className="h-40" />}
          </Section>
        )}
      </div>
      {!isUnknown && (
        <Section
          title={t("riven-market")}
          aside={
            market?.sell && t("riven-lots", { lots: market.sell.lots, basis: market.sell.basis })
          }
        >
          {market === undefined ? (
            <TileGrid least={224} className="gap-2">
              {Array.from({ length: 4 }, (_, at) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: the placeholders are identical
                <Skeleton key={at} className="h-24" />
              ))}
            </TileGrid>
          ) : market.analogs.length === 0 ? (
            <span className="text-caption text-dim">{t("riven-no-analogs")}</span>
          ) : (
            <TileGrid least={224} className="gap-2">
              {market.analogs.map((analog) => (
                <AnalogCard key={analog.id} analog={analog} held={holds} />
              ))}
            </TileGrid>
          )}
        </Section>
      )}
    </div>
  );
}
