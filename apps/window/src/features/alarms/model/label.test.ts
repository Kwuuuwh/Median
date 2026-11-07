import { translate } from "@median/i18n";
import { describe, expect, it } from "vitest";
import { alarmPlace, watchLabel } from "./label";

const t = (id: Parameters<typeof translate>[1], args?: Parameters<typeof translate>[2]) =>
  translate("ru", id, args);

describe("watchLabel", () => {
  it("names each kind of watch in a short line", () => {
    expect(watchLabel(t, { kind: "phase", cycle: "cetus", phase: "night" })).toBe("Цетус: ночь");
    expect(watchLabel(t, { kind: "arrival", vendor: "baro" })).toBe("Баро Ки'Тир: приезд");
    expect(watchLabel(t, { kind: "fissure", era: "axi", mission: null, of: "hard" })).toBe(
      "Акси · Стальной путь",
    );
    expect(watchLabel(t, { kind: "fissure", era: null, mission: "MT_DEFENSE", of: "hard" })).toBe(
      "Оборона · Любая эра · Стальной путь",
    );
  });
});

describe("alarmPlace", () => {
  it("names the mission, node and planet", () => {
    expect(
      alarmPlace({
        node: "SolNode1",
        name: "Galatea",
        planet: "Нептун",
        mission: "Захват",
        missionKey: "Capture",
      }),
    ).toBe("Захват · Galatea, Нептун");
  });
});
