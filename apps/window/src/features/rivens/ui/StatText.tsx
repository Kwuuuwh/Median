import { useLocale } from "@/shared/data/app-state";
import { cn } from "@/shared/lib/cn";
import { elementPicture } from "../model/element";
import { statText, type Written } from "../model/stat";

/** A stat's in-game text, preceded by the icon of the damage type it adds, if any. */
export function StatText({
  stat,
  isCurse,
  isStruck,
  className,
}: {
  stat: Written;
  isCurse: boolean;
  /** Whether to strike the stat through, for a stat the compared riven lacks. */
  isStruck?: boolean;
  className?: string;
}) {
  const locale = useLocale();
  const picture = elementPicture(stat.tag);
  return (
    <span
      className={cn(
        "inline-flex min-w-0 items-center gap-1 tabular-nums",
        isCurse ? "text-danger" : "text-soft",
        isStruck && "line-through opacity-45",
        className,
      )}
    >
      {picture && <img src={picture} alt="" className="size-3.5 shrink-0" />}
      <span className="truncate">{statText(stat, locale)}</span>
    </span>
  );
}
