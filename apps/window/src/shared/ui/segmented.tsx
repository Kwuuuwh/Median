import { ToggleGroup } from "radix-ui";
import type { ReactNode } from "react";
import { cn } from "@/shared/lib/cn";

/** One choice of a segmented control. */
export type Segment<T extends string> = { value: T; label: ReactNode; title?: string };

/** A row of mutually exclusive choices, one always picked. */
export function Segmented<T extends string>({
  value,
  segments,
  onChange,
  className,
  label,
}: {
  value: T;
  segments: readonly Segment<T>[];
  onChange: (value: T) => void;
  className?: string;
  label: string;
}) {
  return (
    <ToggleGroup.Root
      type="single"
      value={value}
      aria-label={label}
      onValueChange={(next) => next && onChange(next as T)}
      className={cn(
        "inline-flex h-8 items-center gap-0.5 rounded-control border border-line bg-sunken p-0.5",
        className,
      )}
    >
      {segments.map((segment) => (
        <ToggleGroup.Item
          key={segment.value}
          value={segment.value}
          title={segment.title}
          className="inline-flex h-full min-w-7 items-center justify-center gap-1 rounded-sm px-2 text-muted text-small transition-colors hover:text-ink data-[state=on]:bg-accent/20 data-[state=on]:text-ink [&_svg]:size-4"
        >
          {segment.label}
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  );
}
