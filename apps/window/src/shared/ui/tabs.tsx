import { Tabs as Primitive } from "radix-ui";
import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/shared/lib/cn";

export const Tabs = Primitive.Root;
export const TabPanel = Primitive.Content;

/** Row of tab names; the active tab is underlined. */
export function TabList({ className, ...props }: ComponentProps<typeof Primitive.List>) {
  return (
    <Primitive.List
      className={cn("flex items-center gap-1 border-line border-b", className)}
      {...props}
    />
  );
}

/** One tab name, with an optional count beside it. */
export function Tab({
  value,
  count,
  children,
}: {
  value: string;
  count?: number;
  children: ReactNode;
}) {
  return (
    <Primitive.Trigger
      value={value}
      className="-mb-px flex items-baseline gap-1.5 border-transparent border-b-2 px-3 pt-1 pb-2 text-body text-muted transition-colors hover:text-ink data-[state=active]:border-accent data-[state=active]:text-ink"
    >
      {children}
      {count !== undefined && <span className="text-dim text-small tabular-nums">{count}</span>}
    </Primitive.Trigger>
  );
}
