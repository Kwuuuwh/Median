import { cn } from "@/shared/lib/cn";

/** What changed in a release, one line per change. */
export function ChangeList({
  changes,
  className,
}: {
  changes: readonly string[];
  className?: string;
}) {
  return (
    <ul className={cn("grid content-start gap-1 overflow-y-auto text-muted text-small", className)}>
      {changes.map((change, at) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: two commits may share a subject
        <li key={at}>{change}</li>
      ))}
    </ul>
  );
}
