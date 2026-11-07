import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatCompact } from "@/shared/lib/format";
import { Tooltip } from "@/shared/ui/tooltip";

/** Compact held count of an item, such as `×12`, shown beside its name; nothing when 0. */
export function HeldMark({ count, className }: { count: number; className?: string }) {
  const t = useT();
  const locale = useLocale();
  if (count <= 0) return null;
  return (
    <Tooltip hint={t("item-held", { count })}>
      <span
        className={cn(
          "inline-flex h-4 shrink-0 items-center rounded-sm bg-live/15 px-1 font-medium text-caption text-live tabular-nums",
          className,
        )}
      >
        ×{formatCompact(locale, count)}
      </span>
    </Tooltip>
  );
}
