import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/shared/lib/cn";

/** Centred empty state of a page: a title saying what is missing, and at most one action. */
export function EmptyState({ title, action }: { title: string; action?: ReactNode }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
      <div className="text-muted text-title">{title}</div>
      {action}
    </div>
  );
}

/** A muted line inside a panel in place of data that is missing or still loading. */
export function EmptyLine({
  icon: Icon,
  children,
  className,
}: {
  icon?: LucideIcon;
  children: string;
  className?: string;
}) {
  return (
    <div className={cn("flex items-center gap-2 text-dim text-small", className)}>
      {Icon && <Icon className="size-4 shrink-0" />}
      {children}
    </div>
  );
}
