import { describe, expect, it } from "vitest";
import { nameFromKey } from "./name";

describe("nameFromKey", () => {
  it("makes words of a key the catalog does not know", () => {
    expect(nameFromKey("/Lotus/Types/Game/ShipScenes/CorpusShipScene")).toBe("Corpus Ship Scene");
    expect(nameFromKey("/Lotus/Types/Packages/MegaPrimeVault/MPVBansheePrimeSinglePack")).toBe(
      "Banshee Prime Single Pack",
    );
    expect(nameFromKey("/Lotus/Types/Packages/PrimeColorPackA")).toBe("Prime Color Pack A");
  });
});
