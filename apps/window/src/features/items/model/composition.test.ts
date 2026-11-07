import type { Recipe, SetRow } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { compositionOf, partName } from "./composition";

const volt: SetRow = {
  slug: "volt_prime_set",
  name: "Вольт Прайм: Комплект",
  ducats: null,
  vaulted: true,
  built: "/Volt",
  members: ["/Volt/Bp", "/Volt/Helmet"],
  per_set: [1, 1],
};

describe("compositionOf", () => {
  it("walks the whole set first, then its parts, from the set or any part", () => {
    expect(compositionOf("/Volt/Helmet", [volt])).toEqual({
      set: volt,
      members: ["/Volt", "/Volt/Bp", "/Volt/Helmet"],
      fromRecipe: false,
    });
    expect(compositionOf("/Mod", [volt])).toEqual({
      set: null,
      members: ["/Mod"],
      fromRecipe: false,
    });
  });

  it("walks an item no set holds through its recipe, its resources left out", () => {
    const recipe: Recipe = {
      blueprint: "/Gyre/Bp",
      credits: 25_000,
      time: 259_200,
      rush: null,
      parts: [
        { item: "/Gyre/Chassis", name: "Chassis", image: null, count: 1, class: "component" },
        { item: "/OrokinCell", name: "Orokin Cell", image: null, count: 3, class: "resource" },
      ],
    };

    expect(compositionOf("/Gyre", [volt], recipe, "Гайра").members).toEqual([
      "/Gyre",
      "/Gyre/Bp",
      "/Gyre/Chassis",
    ]);
  });

  it("names a part without its set", () => {
    expect(partName("Вольт Прайм: Нейрооптика (Чертеж)", "Вольт Прайм")).toBe(
      "Нейрооптика (Чертеж)",
    );
    expect(partName("Вольт Прайм (Чертеж)", "Вольт Прайм")).toBe("Чертеж");
  });
});
