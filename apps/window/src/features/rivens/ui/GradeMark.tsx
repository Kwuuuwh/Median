import type { RivenGrade } from "@median/ipc";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Tooltip } from "@/shared/ui/tooltip";

const TONE: Record<RivenGrade, string> = {
  perfect: "bg-duc/15 text-duc",
  good: "bg-live/15 text-live",
  potential: "bg-plat/15 text-plat",
  bad: "bg-danger/12 text-danger",
};

/** Grade badge of a riven by its weapon's good rolls; a dash for a weapon without good rolls. */
export function GradeMark({ grade }: { grade: RivenGrade | null }) {
  const t = useT();
  if (grade === null) {
    return (
      <Tooltip hint={t("riven-unvalued")}>
        <span className="shrink-0 rounded-control bg-raised px-2 py-0.5 text-muted text-small">
          —
        </span>
      </Tooltip>
    );
  }
  return (
    <span
      className={cn("shrink-0 rounded-control px-2 py-0.5 font-medium text-small", TONE[grade])}
    >
      {t("riven-grade", { grade })}
    </span>
  );
}
