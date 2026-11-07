import type { RerollOdds } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { bestLock, chanceText, kuvaText } from "./odds";

const odds = (locked: string | null, expectedKuva: number | null): RerollOdds => ({
  locked,
  chance: 0.1,
  expectedKuva,
  betterMean: 100,
});

describe("bestLock", () => {
  it("picks the lock that costs the least kuva and skips the ones never seen to land", () => {
    expect(bestLock([odds(null, 9000), odds("A", 7000), odds("B", null)])?.locked).toBe("A");
    expect(bestLock([odds("B", null)])).toBeUndefined();
  });
});

describe("chanceText", () => {
  it("shows tenths under one percent and a floor under the least told apart", () => {
    expect(chanceText("en", 0.368)).toBe("37%");
    expect(chanceText("en", 0.003)).toBe("0.3%");
    expect(chanceText("en", 0.0004)).toBe("<0.1%");
    expect(chanceText("en", 0)).toBe("0%");
  });
});

describe("kuvaText", () => {
  it("caps kuva out of reach and dashes a roll that never lands", () => {
    expect(kuvaText("en", 3331)).toBe("3,331");
    expect(kuvaText("en", 13_984_960)).toBe(">200,000");
    expect(kuvaText("en", null)).toBe("—");
  });
});
