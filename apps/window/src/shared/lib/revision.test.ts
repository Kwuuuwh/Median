import { describe, expect, it } from "vitest";
import { revisionText } from "./revision";

describe("revisionText", () => {
  const build = {
    revision: 8217,
    hash: "cc465feb9",
    built: Date.UTC(2026, 10, 4, 1, 30) / 1000,
    channel: "stable",
  } as const;

  it("shows the revision, the hash and the build time", () => {
    const shown = revisionText(build, "ru");
    expect(shown.startsWith("r8217 (cc465feb9, ")).toBe(true);
    expect(shown).toContain("2026");
  });

  it("leaves the hash out when the build had none", () => {
    expect(revisionText({ ...build, revision: 0, hash: "" }, "ru").startsWith("r0 (")).toBe(true);
    expect(revisionText({ ...build, hash: "" }, "ru")).not.toContain(", ,");
  });
});
