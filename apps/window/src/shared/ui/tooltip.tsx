import { Tooltip as Primitive } from "radix-ui";
import type { ReactNode } from "react";

export const TooltipProvider = Primitive.Provider;

/** A short hint shown over its trigger on hover or focus. */
export function Tooltip({ hint, children }: { hint: ReactNode; children: ReactNode }) {
  return (
    <Primitive.Root>
      <Primitive.Trigger asChild>{children}</Primitive.Trigger>
      <Primitive.Portal>
        <Primitive.Content
          sideOffset={6}
          className="z-50 rounded-control border border-line bg-raised px-2 py-1 text-small text-soft shadow-black/40 shadow-lg"
        >
          {hint}
        </Primitive.Content>
      </Primitive.Portal>
    </Primitive.Root>
  );
}
