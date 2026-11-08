import { describe, expect, it } from "vitest";
import { releaseName, revisionText } from "./revision";

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

describe("releaseName", () => {
  it("names a release by its revision, or by its version when it has none", () => {
    expect(releaseName({ revision: 17, version: "0.0.17-preview" })).toBe("r17");
    expect(releaseName({ revision: null, version: "1.2.0" })).toBe("1.2.0");
  });
});
