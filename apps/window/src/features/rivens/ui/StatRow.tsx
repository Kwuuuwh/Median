import type { AppraisedStat, StatVerdict } from "@median/ipc";
import { CircleCheck, CircleMinus, CircleX, type LucideIcon } from "lucide-react";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Tooltip } from "@/shared/ui/tooltip";
import { rollLetter } from "../model/stat";
import { StatText } from "./StatText";

const VERDICT_ICON: Record<StatVerdict, LucideIcon> = {
  decisive: CircleCheck,
  wanted: CircleCheck,
  harmless: CircleCheck,
  useless: CircleMinus,
  harmful: CircleX,
};

const VERDICT_TONE: Record<StatVerdict, string> = {
  decisive: "text-live",
  wanted: "text-live/70",
  harmless: "text-live/70",
  useless: "text-muted",
  harmful: "text-danger",
};

/** One riven stat: its verdict for the weapon, in-game text, and roll grade within its range. */
export function StatRow({ stat }: { stat: AppraisedStat }) {
  const t = useT();
  const verdict = stat.verdict;
  const Icon = verdict && VERDICT_ICON[verdict];
  const share = stat.rollShare ?? 0;
  const percent = Math.round(share * 100);
  return (
    <div className="flex items-center gap-2 text-small">
      {verdict && Icon ? (
        <Tooltip hint={t("riven-verdict", { verdict })}>
          <Icon className={cn("size-3.5 shrink-0", VERDICT_TONE[verdict])} aria-hidden />
        </Tooltip>
      ) : (
        <span className="size-3.5 shrink-0" />
      )}
      <StatText stat={stat} isCurse={stat.isCurse} className="flex-1" />
      <Tooltip hint={t("riven-roll", { percent })}>
        <span className="w-3 shrink-0 text-center font-medium text-caption text-muted">
          {rollLetter(share)}
        </span>
      </Tooltip>
    </div>
  );
}
