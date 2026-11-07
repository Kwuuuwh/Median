import type { AppraisedStat } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { rollLetter, statText } from "./stat";

const stat = (form: AppraisedStat["form"], amount: number, label: string): AppraisedStat => ({
  tag: "Tag",
  isCurse: false,
  label,
  form,
  amount,
  rollShare: 0.5,
  verdict: "decisive",
});

describe("statText", () => {
  it("writes each form the way the game does", () => {
    expect(statText(stat("percent", 2.744, "% урона"), "en")).toBe("+274.4% урона");
    expect(statText(stat("percent", -1.224, "% мультивыстрел"), "en")).toBe(
      "−122.4% мультивыстрел",
    );
    expect(statText(stat("multiplier", 0.58, "урона Корпусу"), "en")).toBe("x1.58 урона Корпусу");
    expect(statText(stat("plain", 2.7, "к пронзанию навылет"), "en")).toBe(
      "+2.7 к пронзанию навылет",
    );
  });
});

describe("rollLetter", () => {
  it("cuts the range in thirds", () => {
    expect(rollLetter(0.9)).toBe("A");
    expect(rollLetter(0.5)).toBe("B");
    expect(rollLetter(0.1)).toBe("C");
  });
});
