import { cn } from "@/shared/lib/cn";

/** Pulsing placeholder for a value that is still loading; its size comes from `className`. */
export function Skeleton({ className }: { className?: string }) {
  return <span className={cn("block animate-pulse rounded-control bg-raised", className)} />;
}
