import { type ReactNode, useId } from "react";
import { cn } from "@/shared/lib/cn";
import { Input } from "./input";

/** Field text parsed as a whole number; null when it is not one or lies outside the bounds. */
export function wholeNumber(text: string, least: number, most: number): number | null {
  const value = Number.parseInt(text, 10);
  return Number.isFinite(value) && value >= least && value <= most ? value : null;
}

/** A small labelled number field; the label sits beside it, or above it when stacked. */
export function NumberField({
  label,
  value,
  onChange,
  invalid,
  width,
  min = 1,
  max,
  stacked = false,
  className,
  children,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  invalid: boolean;
  width: string;
  min?: number;
  max?: number;
  stacked?: boolean;
  className?: string;
  children?: ReactNode;
}) {
  const id = useId();
  const field = (
    <Input
      id={id}
      type="number"
      min={min}
      max={max}
      value={value}
      aria-invalid={invalid}
      onChange={(event) => onChange(event.target.value)}
      className={width}
    />
  );
  if (stacked) {
    return (
      <label
        htmlFor={id}
        className={cn("grid content-start gap-1 text-caption text-dim", className)}
      >
        {label}
        <span className="flex items-center gap-1.5 text-muted text-small">
          {field}
          {children}
        </span>
      </label>
    );
  }
  return (
    <label
      htmlFor={id}
      className={cn("flex items-center gap-1.5 text-muted text-small", className)}
    >
      {label}
      {field}
      {children}
    </label>
  );
}
