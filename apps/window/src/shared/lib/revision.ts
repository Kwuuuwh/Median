import type { Build } from "@median/ipc";

/** The build as users see it: `r8217 (cc465feb9, 04.11.2026, 01:30)`. */
export function revisionText(
  build: Pick<Build, "revision" | "hash" | "built">,
  locale: string,
): string {
  const built = new Intl.DateTimeFormat(locale, { dateStyle: "short", timeStyle: "short" }).format(
    build.built * 1000,
  );
  const origin = build.hash ? `${build.hash}, ${built}` : built;
  return `r${build.revision} (${origin})`;
}
