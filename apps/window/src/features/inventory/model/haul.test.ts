import type { HaulLine } from "@median/ipc";
import { describe, expect, it } from "vitest";
import { haulRows, haulTotals } from "./haul";

function line(item: string, count: number, fields: Partial<HaulLine>): HaulLine {
  return {
    item,
    variant: "",
    count,
    name: item.slice(1),
    image: null,
    known: true,
    ducats: null,
    worth: null,
    ...fields,
  };
}

describe("the haul", () => {
  const rows = haulRows([
    line("/Part", 3, { ducats: 45, worth: 6 }),
    line("/Junk", 40, { worth: 0 }),
    line("/Mod", 1, { worth: 30 }),
    line("/Resource", 200, {}),
    line("/New", 1, { known: false }),
  ]);

  it("leaves out what fetches nothing and keeps what the catalog does not know", () => {
    expect(rows.map((row) => row.name)).toEqual(["Mod", "Part", "New"]);
  });

  it("keeps an unknown price apart from a price of nothing", () => {
    expect(rows.find((row) => row.item === "/New")?.platinum).toBeNull();
  });

  it("values every copy at what it is expected to fetch", () => {
    expect(haulTotals(rows)).toEqual({ platinum: 48, ducats: 135 });
  });
});
