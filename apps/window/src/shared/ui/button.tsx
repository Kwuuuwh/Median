import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";
import { cn } from "@/shared/lib/cn";

const button = cva(
  "inline-flex shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-control font-medium transition-colors disabled:pointer-events-none disabled:opacity-40 [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      tone: {
        primary: "bg-accent text-canvas hover:brightness-110",
        secondary: "border border-line bg-raised text-soft hover:border-dim hover:text-ink",
        ghost: "text-muted hover:bg-raised hover:text-ink",
        danger: "border border-line bg-raised text-danger hover:border-danger",
      },
      size: {
        sm: "h-7 px-2.5 text-small",
        md: "h-8 px-3 text-body",
        icon: "size-8",
        "icon-sm": "size-7",
      },
    },
    defaultVariants: { tone: "secondary", size: "md" },
  },
);

/** The window's shared button; `tone` defaults to `secondary`, `size` to `md`. */
export function Button({
  className,
  tone,
  size,
  type = "button",
  ...props
}: ComponentProps<"button"> & VariantProps<typeof button>) {
  return <button type={type} className={cn(button({ tone, size }), className)} {...props} />;
}
