import { ArrowDownWideNarrow, ArrowUpNarrowWide } from "lucide-react";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { firstWay, type Sorting } from "@/shared/lib/sort";
import { type Choice, Select } from "./select";

/** Sort key select with a direction button; a newly picked key starts in its initial direction. */
export function SortSelect<K extends string>({
  value,
  choices,
  onChange,
  label,
  className,
  wayOf = firstWay,
}: {
  value: Sorting<K>;
  choices: readonly Choice<K>[];
  onChange: (value: Sorting<K>) => void;
  label: string;
  className?: string;
  /** Whether a key starts descending when first picked. */
  wayOf?: (by: K) => boolean;
}) {
  const t = useT();
  const Way = value.descending ? ArrowDownWideNarrow : ArrowUpNarrowWide;
  return (
    <div className="inline-flex shrink-0 justify-self-start">
      <Select<K>
        label={label}
        className={cn("rounded-r-none", className)}
        value={value.by}
        onChange={(by) => by !== value.by && onChange({ by, descending: wayOf(by) })}
        choices={choices}
      />
      <button
        type="button"
        aria-label={t("sort-direction", { direction: value.descending ? "down" : "up" })}
        title={t("sort-direction", { direction: value.descending ? "down" : "up" })}
        onClick={() => onChange({ ...value, descending: !value.descending })}
        className="inline-flex size-8 items-center justify-center rounded-r-control border border-line border-l-0 bg-raised text-soft outline-none hover:border-dim hover:text-ink"
      >
        <Way className="size-4" />
      </button>
    </div>
  );
}
