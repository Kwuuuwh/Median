import type { Build, Release } from "@median/ipc";

/** A found release as users see it: `r8217`, or its version as announced when it has no revision. */
export function releaseName(release: Pick<Release, "revision" | "version">): string {
  return release.revision === null ? release.version : `r${release.revision}`;
}

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
