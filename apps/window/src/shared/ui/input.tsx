import type { ComponentProps } from "react";
import { cn } from "@/shared/lib/cn";

/** A single-line text or number field. */
export function Input({ className, ...props }: ComponentProps<"input">) {
  return (
    <input
      className={cn(
        "h-8 w-full min-w-0 rounded-control border border-line bg-sunken px-2.5 text-body text-ink tabular-nums outline-none transition-colors placeholder:text-dim hover:border-dim focus-visible:border-muted focus-visible:outline-none aria-invalid:border-danger",
        className,
      )}
      {...props}
    />
  );
}
