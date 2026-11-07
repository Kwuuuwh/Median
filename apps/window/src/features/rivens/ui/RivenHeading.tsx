import type { Good, Riven, RivenAppraisal } from "@median/ipc";
import { RefreshCw } from "lucide-react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { formatCount } from "@/shared/lib/format";
import { Tooltip } from "@/shared/ui/tooltip";
import { RIVEN_TOP_RANK } from "../model/held";
import { GradeMark } from "./GradeMark";
import { PolarityMark } from "./PolarityMark";

/** Riven heading: weapon, name and grade above polarity, mastery rank, rank and rerolls. */
export function RivenHeading({
  riven,
  rank,
  weapon,
  name,
  appraisal,
}: {
  riven: Riven;
  rank: number;
  weapon: Good | undefined;
  name: string | null | undefined;
  appraisal: RivenAppraisal | null | undefined;
}) {
  const t = useT();
  const locale = useLocale();
  return (
    <div className="flex items-center gap-2">
      <span className="grid min-w-0 flex-1 gap-0.5 leading-tight">
        <span className="truncate text-body">
          <span className="font-medium text-soft">
            {weapon?.name ?? riven.weapon.split("/").pop()}
          </span>
          {name && <span className="ml-1.5 text-muted">{name}</span>}
        </span>
        <span className="flex items-center gap-2 text-caption text-muted tabular-nums">
          <PolarityMark polarity={riven.polarity} />
          <span>{t("riven-mastery", { mastery: riven.mastery })}</span>
          <span>{t("inventory-rank", { rank: rank, max: RIVEN_TOP_RANK })}</span>
          <Tooltip hint={t("riven-rerolls")}>
            <span className="inline-flex items-center gap-0.5">
              <RefreshCw className="size-3" aria-hidden />
              {formatCount(locale, riven.rerolls)}
            </span>
          </Tooltip>
        </span>
      </span>
      {appraisal && <GradeMark grade={appraisal.grade} />}
    </div>
  );
}
