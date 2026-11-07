import type { ComponentProps } from "react";
import { cn } from "@/shared/lib/cn";

/** Tiles in as many equal columns as fit, none narrower than `least` pixels. */
export function TileGrid({
  least,
  className,
  style,
  ...props
}: ComponentProps<"div"> & { least: number }) {
  return (
    <div
      className={cn("grid gap-3", className)}
      style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${least}px, 1fr))`, ...style }}
      {...props}
    />
  );
}
