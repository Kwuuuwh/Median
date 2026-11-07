import type { Good, RivenAppraisal, RivenBand, RivenDemand } from "@median/ipc";
import type { ReactNode } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { Money } from "@/shared/ui/money";
import { Skeleton } from "@/shared/ui/skeleton";
import { Tooltip } from "@/shared/ui/tooltip";
import { bandText } from "../model/band";
import type { HeldRiven } from "../model/held";
import { RivenHeading } from "./RivenHeading";
import { StatRow } from "./StatRow";

const DEMAND_TONE: Record<RivenDemand, string> = {
  lively: "text-live",
  slow: "text-plat",
  dead: "text-danger",
  unclear: "text-muted",
};

/** A labelled value under the riven, with a hint on the label; a placeholder while loading. */
function Figure({
  label,
  hint,
  isCounted,
  children,
}: {
  label: string;
  hint: ReactNode;
  isCounted: boolean;
  children: ReactNode;
}) {
  return (
    <span className="grid gap-0.5">
      <Tooltip hint={hint}>
        <span className="w-fit text-caption text-muted">{label}</span>
      </Tooltip>
      {isCounted ? (
        <span className="flex h-5 items-center">{children}</span>
      ) : (
        <Skeleton className="h-5 w-16" />
      )}
    </span>
  );
}

/** A platinum amount; a dash when null or undefined. */
function Platinum({ amount }: { amount: number | null | undefined }) {
  return <Money currency="platinum" amount={amount ?? null} className="font-semibold text-plat" />;
}

/**
 * Tile of one revealed riven: weapon, name, grade and stats, with its current price and the
 * highest price of a rolled riven of the weapon; a click opens the riven.
 */
export function RivenTile({
  held,
  weapon,
  name,
  appraisal,
  band,
  ceiling,
  demand,
  onOpen,
}: {
  held: HeldRiven;
  weapon: Good | undefined;
  /** In-game name of the riven; null when it cannot be determined. */
  name: string | null | undefined;
  /** Null when median does not appraise rivens of the weapon; undefined while loading. */
  appraisal: RivenAppraisal | null | undefined;
  /** Null when no similar riven is listed; undefined while loading. */
  band: RivenBand | null | undefined;
  /** Highest price of a rolled riven of the weapon; undefined while loading. */
  ceiling: number | null | undefined;
  /** How readily rivens of the weapon sell; undefined while loading. */
  demand: RivenDemand | null | undefined;
  onOpen: () => void;
}) {
  const t = useT();
  const locale = useLocale();
  const number = (value: number | null | undefined) => formatCount(locale, value ?? 0);
  const top =
    band === undefined || ceiling === undefined
      ? undefined
      : band || ceiling !== null
        ? Math.max(band?.high ?? 0, ceiling ?? 0)
        : null;
  return (
    <button
      type="button"
      onClick={onOpen}
      className="flex flex-col gap-2.5 rounded-panel border border-line bg-surface p-3 text-left transition-colors hover:border-dim"
    >
      <RivenHeading
        riven={held.riven}
        rank={held.rank}
        weapon={weapon}
        name={name}
        appraisal={appraisal}
      />
      {appraisal ? (
        <div className="flex w-full flex-col gap-1">
          {appraisal.stats.map((stat) => (
            <StatRow key={`${stat.tag}${stat.isCurse}`} stat={stat} />
          ))}
        </div>
      ) : (
        appraisal === null && <span className="text-caption text-dim">{t("riven-unvalued")}</span>
      )}
      {appraisal !== null && (
        <span className="grid w-full grid-cols-3 gap-3 border-line border-t pt-2">
          <Figure
            label={t("riven-price")}
            hint={band ? bandText(t, band, number) : t("riven-no-analogs")}
            isCounted={band !== undefined}
          >
            <Platinum amount={band?.middle} />
          </Figure>
          <Figure
            label={t("riven-ceiling")}
            hint={t("riven-ceiling-hint")}
            isCounted={top !== undefined}
          >
            <Platinum amount={top} />
          </Figure>
          <Figure
            label={t("riven-demand")}
            hint={t("riven-demand-hint")}
            isCounted={demand !== undefined}
          >
            {demand ? (
              <span className={cn("font-medium text-small", DEMAND_TONE[demand])}>
                {t("riven-demand-value", { demand })}
              </span>
            ) : (
              <span className="text-dim">—</span>
            )}
          </Figure>
        </span>
      )}
    </button>
  );
}
