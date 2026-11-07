import { cn } from "@/shared/lib/cn";
import { Input } from "./input";

/** Lower and upper whole-number fields under one label, each with a hint naming its bound. */
export function RangeField({
  label,
  from,
  to,
  onFrom,
  onTo,
  fromHint,
  toHint,
  invalidFrom,
  invalidTo,
  className,
}: {
  label: string;
  from: string;
  to: string;
  onFrom: (value: string) => void;
  onTo: (value: string) => void;
  fromHint: string;
  toHint: string;
  invalidFrom: boolean;
  invalidTo: boolean;
  className?: string;
}) {
  return (
    <span className={cn("flex items-center gap-1.5 text-muted text-small", className)}>
      {label}
      <Input
        type="number"
        min={0}
        aria-label={`${label} ${fromHint}`}
        placeholder={fromHint}
        value={from}
        aria-invalid={invalidFrom}
        onChange={(event) => onFrom(event.target.value)}
        className="w-16"
      />
      <span className="text-dim">–</span>
      <Input
        type="number"
        min={0}
        aria-label={`${label} ${toHint}`}
        placeholder={toHint}
        value={to}
        aria-invalid={invalidTo}
        onChange={(event) => onTo(event.target.value)}
        className="w-16"
      />
    </span>
  );
}
