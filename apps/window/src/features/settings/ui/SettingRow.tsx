import type { ReactNode } from "react";

/** One setting: its name and an optional hint on the left, its control on the right. */
export function SettingRow({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 border-hairline border-b py-3 last:border-b-0">
      <div className="grid min-w-0 gap-0.5">
        <span className="text-body text-ink">{label}</span>
        {hint && <span className="text-dim text-small">{hint}</span>}
      </div>
      <div className="flex shrink-0 items-center gap-3">{children}</div>
    </div>
  );
}

/** A titled block of settings rows. */
export function SettingSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-1">
      <h2 className="font-medium text-muted text-small">{title}</h2>
      <div className="rounded-panel border border-line bg-surface px-4">{children}</div>
    </section>
  );
}
