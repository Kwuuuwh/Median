import { Toggle as Primitive } from "radix-ui";
import type { ComponentProps } from "react";
import { cn } from "@/shared/lib/cn";

/** A button that stays pressed: on or off. */
export function Toggle({ className, ...props }: ComponentProps<typeof Primitive.Root>) {
  return (
    <Primitive.Root
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-control border border-line px-2.5 text-muted text-small transition-colors hover:text-ink data-[state=on]:border-accent/60 data-[state=on]:bg-accent/10 data-[state=on]:text-ink [&_svg]:size-3.5",
        className,
      )}
      {...props}
    />
  );
}
