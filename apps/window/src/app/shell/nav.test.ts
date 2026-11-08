import { describe, expect, it } from "vitest";
import { NAV, shows } from "./nav";

const entry = (to: string) => {
  const found = NAV.flatMap((group) => group.entries).find((each) => each.to === to);
  if (!found) throw new Error(`no link to ${to}`);
  return found;
};

describe("shows", () => {
  it("hides the catalog from stable builds only", () => {
    const catalog = entry("/catalog");
    expect(shows(catalog, "stable")).toBe(false);
    expect(shows(catalog, "preview")).toBe(true);
    expect(shows(catalog, "dev")).toBe(true);
  });

  it("shows rivens in dev builds only", () => {
    const rivens = entry("/rivens");
    expect(shows(rivens, "stable")).toBe(false);
    expect(shows(rivens, "preview")).toBe(false);
    expect(shows(rivens, "dev")).toBe(true);
  });

  it("shows open links at once and limited ones once the channel is known", () => {
    expect(shows(entry("/timers"), undefined)).toBe(true);
    expect(shows(entry("/catalog"), undefined)).toBe(false);
  });
});
