import { Popover as Primitive } from "radix-ui";
import type { ComponentProps } from "react";
import { cn } from "@/shared/lib/cn";

export const Popover = Primitive.Root;
export const PopoverTrigger = Primitive.Trigger;

/** The floating panel a popover opens. */
export function PopoverContent({
  className,
  align = "start",
  sideOffset = 6,
  ...props
}: ComponentProps<typeof Primitive.Content>) {
  return (
    <Primitive.Portal>
      <Primitive.Content
        align={align}
        sideOffset={sideOffset}
        className={cn(
          "data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 z-50 rounded-panel border border-line bg-raised p-3 shadow-black/40 shadow-xl outline-none data-[state=closed]:animate-out data-[state=open]:animate-in",
          className,
        )}
        {...props}
      />
    </Primitive.Portal>
  );
}
