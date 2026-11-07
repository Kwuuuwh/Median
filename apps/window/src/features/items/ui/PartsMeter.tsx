import { cn } from "@/shared/lib/cn";

/** One dot per set part, filled for each part held, followed by `owned/total`. */
export function PartsMeter({ owned, total }: { owned: number; total: number }) {
  return (
    <span className="inline-flex items-center gap-1.5 text-small text-soft tabular-nums">
      <span className="inline-flex gap-0.5" aria-hidden>
        {Array.from({ length: total }, (_, index) => (
          <span
            // biome-ignore lint/suspicious/noArrayIndexKey: the dots are identical
            key={index}
            className={cn(
              "size-1.5 rounded-full",
              index < owned ? "bg-live" : "border border-dim/60",
            )}
          />
        ))}
      </span>
      {owned}/{total}
    </span>
  );
}
