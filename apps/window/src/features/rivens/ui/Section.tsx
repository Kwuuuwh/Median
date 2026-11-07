import type { ReactNode } from "react";

/** A titled section of the riven sheet. */
export function Section({
  title,
  aside,
  children,
}: {
  title: string;
  /** Optional content at the right end of the title. */
  aside?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="flex min-w-0 flex-col gap-2">
      <header className="flex items-baseline gap-2 border-line border-b pb-1.5">
        <span className="flex-1 font-medium text-caption text-muted uppercase tracking-wide">
          {title}
        </span>
        {aside && <span className="text-caption text-dim">{aside}</span>}
      </header>
      {children}
    </section>
  );
}
