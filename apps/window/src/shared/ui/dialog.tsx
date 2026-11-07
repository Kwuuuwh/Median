import { Dialog as Primitive } from "radix-ui";
import type { ComponentProps } from "react";
import { cn } from "@/shared/lib/cn";

export const Dialog = Primitive.Root;
export const DialogClose = Primitive.Close;
export const DialogTrigger = Primitive.Trigger;

/** Dialog placement: `form` centred in the window, `sheet` as a tall panel near the top. */
const PLACES = {
  form: { panel: "top-1/2 -translate-y-1/2 p-5", backdrop: "bg-black/50" },
  sheet: { panel: "top-[6vh] flex max-h-[88vh] flex-col", backdrop: "bg-black/60" },
} as const;

/** A dialog's panel over the dimmed window; its width and inner layout come from `className`. */
export function DialogContent({
  place = "form",
  className,
  ...props
}: ComponentProps<typeof Primitive.Content> & { place?: keyof typeof PLACES }) {
  return (
    <Primitive.Portal>
      <Primitive.Overlay
        className={cn(
          "data-[state=open]:fade-in-0 fixed inset-0 z-40 data-[state=open]:animate-in",
          PLACES[place].backdrop,
        )}
      />
      <Primitive.Content
        className={cn(
          "fixed left-1/2 z-50 -translate-x-1/2 rounded-panel border border-line bg-surface shadow-2xl shadow-black/50 outline-none",
          PLACES[place].panel,
          className,
        )}
        {...props}
      />
    </Primitive.Portal>
  );
}

/** Screen-reader title every dialog has; `sr-only` when the content already shows a name. */
export function DialogTitle({ className, ...props }: ComponentProps<typeof Primitive.Title>) {
  return (
    <Primitive.Title
      className={cn("font-display font-semibold text-title", className)}
      {...props}
    />
  );
}

/** Description line under the title; screen readers announce it with the title. */
export function DialogDescription({
  className,
  ...props
}: ComponentProps<typeof Primitive.Description>) {
  return <Primitive.Description className={cn("text-muted text-small", className)} {...props} />;
}
