import { useState } from "react";
import { OrderTile } from "@/features/orders/ui/OrderTile";
import { useAppState } from "@/shared/data/app-state";
import { useOwnOrders } from "@/shared/data/market";
import { useRivenBuys } from "@/shared/data/rivens";
import { useFigures, usePositions } from "@/shared/data/trading";
import { useT } from "@/shared/i18n";
import { EmptyState } from "@/shared/ui/empty-state";
import { Segmented } from "@/shared/ui/segmented";
import { Tab, TabList, Tabs } from "@/shared/ui/tabs";
import { TileGrid } from "@/shared/ui/tile-grid";
import { agentBids, tiedBy } from "../model/agent";
import { AgentBar } from "./AgentBar";
import { PositionTile } from "./PositionTile";
import { RivensView } from "./RivensView";
import { StatsView } from "./StatsView";

type AgentTab = "bids" | "positions" | "rivens" | "stats";

/** Statistics periods in days; `0` covers all time. */
const PERIODS = ["7", "30", "0"] as const;
type Period = (typeof PERIODS)[number];

/**
 * Agent page: the summary bar, then tabs of buy orders, positions held for resale, rivens, and
 * trading statistics over a chosen period.
 */
export function AgentPage() {
  const t = useT();
  const orders = useOwnOrders().data ?? [];
  const positions = usePositions().data ?? [];
  const [tab, setTab] = useState<AgentTab>("bids");
  const [period, setPeriod] = useState<Period>("30");
  const deals = useFigures(Number(period)).data?.deals ?? [];
  const bids = agentBids(orders);
  const state = useAppState();
  const purse = state?.account.platinum ?? null;
  const picks = state?.agent.picks ?? [];
  const buys = useRivenBuys().data ?? [];

  const periodPicker = (
    <Segmented<Period>
      label={t("agent-stats")}
      value={period}
      onChange={setPeriod}
      segments={PERIODS.map((days) => ({
        value: days,
        label: t("trade-period", { days: Number(days) }),
      }))}
    />
  );

  return (
    <Tabs
      value={tab}
      onValueChange={(value) => setTab(value as AgentTab)}
      className="relative flex min-w-0 flex-1 flex-col"
    >
      <AgentBar orders={orders} />
      <div className="flex shrink-0 items-end gap-4 border-line border-b px-6 pt-3">
        <TabList className="shrink-0 border-b-0">
          <Tab value="bids" count={bids.length}>
            {t("agent-bids")}
          </Tab>
          <Tab value="positions" count={positions.length}>
            {t("trade-positions")}
          </Tab>
          <Tab value="rivens" count={picks.length}>
            {t("agent-rivens")}
          </Tab>
          <Tab value="stats">{t("agent-stats")}</Tab>
        </TabList>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-4 pb-16">
        {tab === "bids" &&
          (bids.length === 0 ? (
            <EmptyState title={t("agent-no-bids")} />
          ) : (
            <TileGrid least={208}>
              {bids.map((bid) => (
                <OrderTile key={bid.id} order={bid} />
              ))}
            </TileGrid>
          ))}
        {tab === "positions" &&
          (positions.length === 0 ? (
            <EmptyState title={t("trade-no-positions")} />
          ) : (
            <TileGrid least={208}>
              {[...positions]
                .sort((a, b) => b.days - a.days)
                .map((position) => (
                  <PositionTile key={`${position.slug}:${position.variant}`} position={position} />
                ))}
            </TileGrid>
          ))}
        {tab === "rivens" && (
          <RivensView
            picks={picks}
            buys={buys}
            isOn={state?.policy.rivens ?? false}
            days={Number(period)}
            period={periodPicker}
          />
        )}
        {tab === "stats" && (
          <StatsView
            deals={deals}
            positions={positions}
            bids={bids.length}
            tied={tiedBy(bids)}
            purse={purse}
            days={Number(period)}
            period={periodPicker}
          />
        )}
      </div>
    </Tabs>
  );
}
