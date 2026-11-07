import type { Good } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { type Snapshot, snapshotAnswers } from "./snapshot";

function good(item: string, name: string): Good {
  return {
    item,
    name,
    kind: "",
    image: null,
    known: true,
    slug: null,
    tradable: null,
    vaulted: null,
    prime: false,
    ducats: null,
    makes: null,
    mastered: null,
    collection: null,
  };
}

const snapshot: Snapshot = {
  getSets: [
    { args: ["ru"], answer: [] },
    { args: ["en"], answer: [] },
  ],
  getGoods: [
    { args: ["ru", ["/A", "/B"]], answer: [good("/A", "Эй"), good("/B", "Би")] },
    { args: ["en", ["/A", "/B"]], answer: [good("/A", "A"), good("/B", "B")] },
  ],
};

describe("snapshotAnswers", () => {
  const answers = snapshotAnswers(snapshot);

  it("replays the call recorded with the same arguments", () => {
    expect(answers.getSets?.("en")).toEqual([]);
  });

  it("refuses arguments that were never recorded", () => {
    expect(() => answers.getSets?.("de")).toThrow();
  });

  it("picks the asked goods out of the recorded ones", () => {
    expect(answers.getGoods?.("en", ["/B"])).toEqual([good("/B", "B")]);
  });

  it("names an item in the other language", () => {
    expect(answers.getOtherName?.("ru", "/A")).toBe("A");
  });
});
