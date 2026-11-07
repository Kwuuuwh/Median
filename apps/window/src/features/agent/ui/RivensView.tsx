import type { Good, Riven, RivenBuy, RivenPick } from "@median/ipc";
import type { ReactNode } from "react";
import { useGoods } from "@/shared/data/catalog";
import { useRivenAppraisals, useRivenNames } from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { DAY } from "@/shared/lib/time";
import { useNow } from "@/shared/lib/use-now";
import { EmptyState } from "@/shared/ui/empty-state";
import { Money } from "@/shared/ui/money";
import { TileGrid } from "@/shared/ui/tile-grid";
import { rivenFigures } from "../model/rivens";
import { BoughtTile } from "./BoughtTile";
import { PickTile } from "./PickTile";
import { Figure, Signed } from "./StatsView";

/** A titled group of tiles with its count. */
function Shelf({ title, count, children }: { title: string; count: number; children: ReactNode }) {
  return (
    <section className="grid gap-2">
      <h3 className="flex items-baseline gap-2 text-muted text-small">
        {title}
        <span className="text-dim tabular-nums">{count}</span>
      </h3>
      {children}
    </section>
  );
}

/** The agent's rivens: purchase and sale totals, auctions it picked, and unsold rivens bought. */
export function RivensView({
  picks,
  buys,
  isOn,
  days,
  period,
}: {
  picks: readonly RivenPick[];
  buys: readonly RivenBuy[];
  isOn: boolean;
  days: number;
  period: ReactNode;
}) {
  const t = useT();
  const now = useNow();
  const held = buys.filter((buy) => buy.sold === null);
  const rivens: { riven: Riven; rank: number }[] = [...picks, ...held];
  const goods = new Map(
    (useGoods([...new Set(rivens.map(({ riven }) => riven.weapon))]).data ?? []).map(
      (good: Good) => [good.item, good],
    ),
  );
  const names = useRivenNames(rivens.map(({ riven }) => riven)).data ?? [];
  const appraisals = useRivenAppraisals(rivens);
  const figures = rivenFigures(buys, days === 0 ? 0 : now - days * DAY);
  const at = (index: number) => ({
    name: names[index],
    appraisal: appraisals[index],
  });

  return (
    <div className="grid gap-5">
      <div className="flex items-start gap-3">
        <TileGrid least={176} className="flex-1">
          <Figure
            label={t("riven-figure-bought")}
            hint={<Money currency="platinum" amount={figures.spent} />}
          >
            {figures.bought}
          </Figure>
          <Figure
            label={t("riven-figure-sold")}
            hint={<Money currency="platinum" amount={figures.earned} />}
          >
            {figures.sold}
          </Figure>
          <Figure label={t("riven-figure-profit")}>
            <Signed amount={figures.profit} />
          </Figure>
          <Figure label={t("riven-figure-held")}>
            <Money currency="platinum" amount={figures.held} />
          </Figure>
        </TileGrid>
        {period}
      </div>
      <Shelf title={t("riven-picks")} count={picks.length}>
        {picks.length === 0 ? (
          <EmptyState title={t(isOn ? "riven-picks-none" : "riven-picks-off")} />
        ) : (
          <TileGrid least={320} className="items-start">
            {picks.map((pick, index) => (
              <PickTile
                key={pick.id}
                pick={pick}
                weapon={goods.get(pick.riven.weapon)}
                {...at(index)}
              />
            ))}
          </TileGrid>
        )}
      </Shelf>
      {held.length > 0 && (
        <Shelf title={t("riven-held")} count={held.length}>
          <TileGrid least={320} className="items-start">
            {held.map((buy, index) => (
              <BoughtTile
                key={buy.auction}
                buy={buy}
                weapon={goods.get(buy.riven.weapon)}
                {...at(picks.length + index)}
              />
            ))}
          </TileGrid>
        </Shelf>
      )}
    </div>
  );
}
