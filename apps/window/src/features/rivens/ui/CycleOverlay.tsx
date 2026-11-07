import type {
  CycleCard,
  CycleRoll,
  RerollOdds,
  RivenAppraisal,
  RivenBand,
  RivenChoices,
} from "@median/ipc";
import { CircleAlert, LoaderCircle } from "lucide-react";
import type { ReactNode } from "react";
import { useAppState, useLocale } from "@/shared/data/app-state";
import {
  useRivenAppraisals,
  useRivenBands,
  useRivenChoices,
  useRivenRolls,
} from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCount } from "@/shared/lib/format";
import { CurrencyIcon } from "@/shared/ui/currency-icon";
import { bandText } from "../model/band";
import { sellsBetter, type Weighed } from "../model/grade";
import { AnalogCard } from "./AnalogCard";
import { GradeMark } from "./GradeMark";
import { RollTable } from "./RollTable";
import { StatRow } from "./StatRow";

/** Empty side margins of a 1920×1080 cycling screen that hold the columns, as `[x, width]` px. */
const MARGINS: Record<"old" | "fresh", [number, number]> = {
  old: [24, 430],
  fresh: [1466, 430],
};

/** Top and bottom of both columns on a 1920×1080 screen, in pixels. */
const SPAN: [number, number] = [112, 975];

/** Similar auctions shown under each riven. */
const LOTS = 6;

/** Empty middle area above the cards that holds the reroll panel, as `[x, y, width]` px. */
const ROLLING: [number, number, number] = [735, 112, 450];

const PANEL = "rounded-xl border bg-black/80 p-3 text-soft backdrop-blur-sm";

const share = (part: number, whole: number) => `${(part / whole) * 100}%`;

/** A small spinner with a label of what is loading. */
function Waiting({ children }: { children: ReactNode }) {
  return (
    <span className="inline-flex items-center gap-1.5 text-muted text-small">
      <LoaderCircle className="size-3.5 animate-spin" />
      {children}
    </span>
  );
}

/** One riven's panel: name and grade, price band of similar rivens, and stats. */
function RivenPanel({
  card,
  appraisal,
  sell,
  isBetter,
}: {
  card: CycleCard;
  appraisal: RivenAppraisal | null | undefined;
  /** Undefined while loading. */
  sell: RivenBand | null | undefined;
  isBetter: boolean;
}) {
  const t = useT();
  const locale = useLocale();
  const number = (value: number | null | undefined) =>
    value == null ? "—" : formatCount(locale, Math.round(value));
  return (
    <section
      className={cn(
        PANEL,
        "grid gap-1.5",
        isBetter ? "border-accent shadow-[0_0_20px] shadow-accent/40" : "border-white/10",
      )}
    >
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate font-medium text-ink text-title">{card.name}</span>
        {appraisal && <GradeMark grade={appraisal.grade} />}
      </div>
      {sell === undefined ? (
        <Waiting>{t("riven-cycle-pricing")}</Waiting>
      ) : sell ? (
        <span className="inline-flex items-center gap-1 text-body tabular-nums">
          <CurrencyIcon currency="platinum" />
          <span className="font-semibold text-ink">{number(sell.middle)}</span>
          <span className="text-muted text-small">{bandText(t, sell, number)}</span>
        </span>
      ) : null}
      {appraisal === undefined ? (
        <Waiting>{t("riven-cycle-appraising")}</Waiting>
      ) : (
        appraisal?.stats.map((stat) => <StatRow key={`${stat.tag}${stat.isCurse}`} stat={stat} />)
      )}
    </section>
  );
}

/** Reroll panel: odds per lock option with the best one marked, and the rolls made so far. */
function RollPanel({
  odds,
  appraisal,
  rolls,
}: {
  /** Null when the riven cannot be priced; undefined while loading. */
  odds: readonly RerollOdds[] | null | undefined;
  appraisal: RivenAppraisal | null | undefined;
  rolls: readonly CycleRoll[];
}) {
  const t = useT();
  const locale = useLocale();
  return (
    <section className={cn(PANEL, "grid gap-2 border-white/10")}>
      <span className="font-medium text-body text-ink">{t("riven-reroll")}</span>
      {odds === undefined || appraisal === undefined ? (
        <Waiting>{t("riven-cycle-pricing")}</Waiting>
      ) : (
        odds && appraisal && <RollTable odds={odds} stats={appraisal.stats} />
      )}
      {rolls.length > 0 && (
        <div className="grid gap-0.5 border-white/10 border-t pt-2 text-small tabular-nums">
          {rolls.map((roll, at) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: rolls are keyed by position
            <span key={at} className="flex items-center gap-2">
              <span className="w-12 text-right text-muted">{formatCount(locale, roll.kuva)}</span>
              {roll.name === null ? (
                <LoaderCircle className="size-3.5 animate-spin text-muted" />
              ) : (
                <span className="truncate text-soft">{roll.name}</span>
              )}
            </span>
          ))}
        </div>
      )}
    </section>
  );
}

/** Similar riven auctions on the market; stats the riven lacks are struck through. */
function LotsPanel({
  card,
  choices,
}: {
  card: CycleCard;
  choices: RivenChoices | null | undefined;
}) {
  const t = useT();
  const held = new Set([
    ...card.riven.buffs.map((stat) => `${stat.tag}false`),
    ...card.riven.curses.map((stat) => `${stat.tag}true`),
  ]);
  const lots = choices?.analogs.slice(0, LOTS) ?? [];
  return (
    <section className={cn(PANEL, "grid min-h-0 gap-2 border-white/10")}>
      <span className="font-medium text-body text-ink">{t("riven-market")}</span>
      {choices === undefined ? (
        <Waiting>{t("riven-cycle-pricing")}</Waiting>
      ) : (
        <div className="grid grid-cols-2 gap-2">
          {lots.map((lot) => (
            <AnalogCard key={lot.id} analog={lot} held={held} />
          ))}
        </div>
      )}
    </section>
  );
}

/** A column placed in one empty side margin of the screen. */
function Column({ margin, children }: { margin: [number, number]; children: ReactNode }) {
  const [x, width] = margin;
  return (
    <div
      className="absolute flex flex-col gap-3 overflow-hidden"
      style={{
        left: share(x, 1920),
        width: share(width, 1920),
        top: share(SPAN[0], 1080),
        bottom: share(1080 - SPAN[1], 1080),
      }}
    >
      {children}
    </div>
  );
}

/**
 * Overlay over the riven cycling screen: the current riven on the left, the newly rolled one on
 * the right, reroll odds in the middle; renders nothing while the screen is closed.
 */
export function CycleOverlay() {
  const t = useT();
  const cycling = useAppState()?.game.cycling;
  const cards = cycling?.state === "open" ? cycling.cards : [];
  const old = cards.find((card) => !card.fresh);
  const fresh = cards.find((card) => card.fresh);
  const shown = [old, fresh].filter((card): card is CycleCard => card !== undefined);
  const appraisals = useRivenAppraisals(shown);
  const kept = useRivenBands(shown.map((card) => card.riven));
  const oldChoices = useRivenChoices(old?.riven ?? null).data;
  const freshChoices = useRivenChoices(fresh?.riven ?? null).data;
  const oldRolls = useRivenRolls(old?.riven ?? null).data;
  if (!cycling || cycling.state === "gone") return null;

  const oldAppraisal = old ? appraisals[0] : undefined;
  const freshAppraisal = fresh ? appraisals[old ? 1 : 0] : undefined;
  const sellOf = (choices: RivenChoices | null | undefined, at: number) =>
    choices === undefined ? (kept[at] ?? undefined) : (choices?.sell ?? null);
  const oldSell = old ? sellOf(oldChoices, 0) : undefined;
  const freshSell = fresh ? sellOf(freshChoices, old ? 1 : 0) : undefined;
  const weighed = (
    sell: RivenBand | null | undefined,
    appraisal: RivenAppraisal | null | undefined,
  ): Weighed => ({ grade: appraisal?.grade, price: sell?.middle });
  const freshIsBetter =
    fresh === undefined
      ? null
      : sellsBetter(weighed(freshSell, freshAppraisal), weighed(oldSell, oldAppraisal));

  return (
    <div className="pointer-events-none fixed inset-0">
      <div className="absolute inset-0 m-auto aspect-video max-h-full max-w-full">
        {old && (
          <Column margin={MARGINS.old}>
            <RivenPanel
              card={old}
              appraisal={oldAppraisal}
              sell={oldSell}
              isBetter={freshIsBetter === false}
            />
            <LotsPanel card={old} choices={oldChoices} />
          </Column>
        )}
        {fresh && (
          <Column margin={MARGINS.fresh}>
            <RivenPanel
              card={fresh}
              appraisal={freshAppraisal}
              sell={freshSell}
              isBetter={freshIsBetter === true}
            />
            <LotsPanel card={fresh} choices={freshChoices} />
          </Column>
        )}
        {old && (
          <div
            className="absolute"
            style={{
              left: share(ROLLING[0], 1920),
              top: share(ROLLING[1], 1080),
              width: share(ROLLING[2], 1920),
            }}
          >
            <RollPanel odds={oldRolls} appraisal={oldAppraisal} rolls={cycling.rolls} />
          </div>
        )}
        {cycling.reading ? (
          <div className="absolute top-[4%] left-1/2 flex -translate-x-1/2 items-center gap-2 rounded-xl border border-white/10 bg-black/80 px-4 py-2.5 text-sm text-soft backdrop-blur-sm">
            <LoaderCircle className="size-4 animate-spin" />
            {t("riven-cycle-reading")}
          </div>
        ) : (
          cycling.unread &&
          shown.length === 0 && (
            <div className="absolute top-[4%] left-1/2 flex -translate-x-1/2 items-center gap-2 rounded-xl border border-white/10 bg-black/80 px-4 py-2.5 text-sm text-warn backdrop-blur-sm">
              <CircleAlert className="size-4" />
              {t("riven-cycle-unread", { why: cycling.unread })}
            </div>
          )
        )}
      </div>
    </div>
  );
}
