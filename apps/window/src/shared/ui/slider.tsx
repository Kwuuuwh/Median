import { Slider as Primitive } from "radix-ui";
import type { ComponentProps } from "react";
import { cn } from "@/shared/lib/cn";

/** A horizontal slider with one thumb. */
export function Slider({ className, ...props }: ComponentProps<typeof Primitive.Root>) {
  return (
    <Primitive.Root
      className={cn(
        "relative flex h-5 touch-none select-none items-center data-[disabled]:opacity-40",
        className,
      )}
      {...props}
    >
      <Primitive.Track className="relative h-1 grow overflow-hidden rounded-full bg-raised">
        <Primitive.Range className="absolute h-full bg-accent" />
      </Primitive.Track>
      <Primitive.Thumb className="block size-3.5 rounded-full border border-line bg-ink outline-none focus-visible:ring-2 focus-visible:ring-ink/40" />
    </Primitive.Root>
  );
}
