import type { Good } from "@median/ipc";
import { useState } from "react";
import { useGoods } from "@/shared/data/catalog";
import { useHoldings } from "@/shared/data/holdings";
import {
  useRivenAppraisals,
  useRivenBands,
  useRivenCeilings,
  useRivenDemands,
  useRivenNames,
} from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { matcher } from "@/shared/lib/search";
import { useKept } from "@/shared/lib/use-kept";
import { EmptyState } from "@/shared/ui/empty-state";
import { SearchField } from "@/shared/ui/search-field";
import { Select } from "@/shared/ui/select";
import { TileGrid } from "@/shared/ui/tile-grid";
import { gradeRank } from "../model/grade";
import { type HeldRiven, heldRivens } from "../model/held";
import { HeldDialog } from "./HeldDialog";
import { RivenTile } from "./RivenTile";
import { VeiledTile } from "./VeiledTile";

type RivenOrder = "grade" | "weapon" | "rerolls";

const ORDERS = [
  ["grade", "rivens-by-grade"],
  ["weapon", "rivens-by-weapon"],
  ["rerolls", "rivens-by-rerolls"],
] as const;

/** Rivens page: every revealed riven the account holds as a tile, veiled ones counted by class. */
export function RivensPage() {
  const t = useT();
  const pieces = useHoldings().data?.pieces ?? [];
  const { revealed, veiled } = heldRivens(pieces);
  const weapons = [...new Set(revealed.map((held) => held.riven.weapon))].sort();
  const goods = new Map((useGoods(weapons).data ?? []).map((good: Good) => [good.item, good]));
  const [search, setSearch] = useKept("rivens.search", "");
  const [order, setOrder] = useKept<RivenOrder>("rivens.sort", "grade");
  const [opened, setOpened] = useState<string | null>(null);
  const appraisals = useRivenAppraisals(revealed);
  const names = useRivenNames(revealed.map((held) => held.riven)).data ?? [];
  const named = new Map(revealed.map((held, at) => [held.key, names[at]]));
  const appraised = new Map(revealed.map((held, at) => [held.key, appraisals[at]]));
  const bands = useRivenBands(revealed.map((held) => held.riven));
  const banded = new Map(revealed.map((held, at) => [held.key, bands[at]]));
  const ceilings = useRivenCeilings(weapons);
  const ceilingOf = new Map(weapons.map((weapon, at) => [weapon, ceilings[at]]));
  const demands = useRivenDemands(revealed.map((held) => held.riven));
  const demandOf = new Map(revealed.map((held, at) => [held.key, demands[at]]));
  const standingOf = (held: HeldRiven) => gradeRank(appraised.get(held.key)?.grade);

  const nameOf = (held: HeldRiven) => goods.get(held.riven.weapon)?.name ?? held.riven.weapon;
  const matches = matcher(search);
  const shown = revealed
    .filter((held) => matches(`${nameOf(held)} ${named.get(held.key) ?? ""}`))
    .sort((one, other) => {
      if (order === "grade") return standingOf(other) - standingOf(one);
      if (order === "rerolls") return other.riven.rerolls - one.riven.rerolls;
      return nameOf(one).localeCompare(nameOf(other));
    });

  const openedRiven = revealed.find((held) => held.key === opened);

  return (
    <div className="relative flex min-w-0 flex-1 flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 border-line border-b px-6 py-3">
        <span className="flex-1" />
        <SearchField value={search} onChange={setSearch} placeholder={t("rivens-search")} />
        <Select<RivenOrder>
          label={t("rivens-sort")}
          className="w-40"
          value={order}
          onChange={setOrder}
          choices={ORDERS.map(([value, label]) => ({ value, label: t(label) }))}
        />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-4 pb-16">
        {shown.length === 0 && veiled.length === 0 ? (
          <EmptyState title={t("rivens-none")} />
        ) : (
          <TileGrid least={320} className="items-start">
            {shown.map((held) => (
              <RivenTile
                key={held.key}
                held={held}
                weapon={goods.get(held.riven.weapon)}
                name={named.get(held.key)}
                appraisal={appraised.get(held.key)}
                band={banded.get(held.key)}
                ceiling={ceilingOf.get(held.riven.weapon)}
                demand={demandOf.get(held.key)}
                onOpen={() => setOpened(held.key)}
              />
            ))}
            {veiled.length > 0 && <VeiledTile veiled={veiled} />}
          </TileGrid>
        )}
      </div>
      <HeldDialog
        held={openedRiven ?? null}
        weapon={openedRiven && goods.get(openedRiven.riven.weapon)}
        name={openedRiven && named.get(openedRiven.key)}
        appraisal={openedRiven && appraised.get(openedRiven.key)}
        onClose={() => setOpened(null)}
      />
    </div>
  );
}
