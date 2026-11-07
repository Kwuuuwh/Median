import type { ReactNode } from "react";
import { cn } from "@/shared/lib/cn";
import { Money } from "@/shared/ui/money";
import { HeldMark } from "./HeldMark";
import { ItemThumb } from "./ItemThumb";

/**
 * One row of a short item list: picture, name, held count, price, and optional trailing content;
 * a `wanted` item is highlighted.
 */
export function ItemLine({
  image,
  name,
  own,
  wanted = false,
  platinum,
  lead,
  onOpen,
  children,
}: {
  image: string | null;
  name: string;
  own: number;
  wanted?: boolean;
  platinum: number | null;
  lead?: ReactNode;
  /** Opens the item's card. */
  onOpen?: () => void;
  children?: ReactNode;
}) {
  const item = (
    <>
      <ItemThumb image={image} size="sm" />
      <span
        className={cn(
          "min-w-0 flex-1 text-body leading-tight transition-colors",
          wanted ? "font-medium text-ink" : own > 0 ? "text-muted" : "text-soft",
        )}
      >
        {name}
      </span>
      <HeldMark count={own} />
    </>
  );
  return (
    <li className="flex items-center gap-2.5 py-1">
      {lead}
      {onOpen ? (
        <button
          type="button"
          onClick={onOpen}
          className="flex min-w-0 flex-1 items-center gap-2.5 text-left hover:[&>span]:text-accent"
        >
          {item}
        </button>
      ) : (
        <span className="flex min-w-0 flex-1 items-center gap-2.5">{item}</span>
      )}
      <Money currency="platinum" amount={platinum} className="shrink-0 text-body text-ink" />
      {children}
    </li>
  );
}
