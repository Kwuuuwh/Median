import { SyndicatesPanel } from "@/features/syndicates/ui/SyndicatesPanel";
import { useAppState } from "@/shared/data/app-state";
import { useNodes } from "@/shared/data/catalog";
import { useT } from "@/shared/i18n";
import { MINUTE } from "@/shared/lib/time";
import { useKept } from "@/shared/lib/use-kept";
import { useNow } from "@/shared/lib/use-now";
import { EmptyState } from "@/shared/ui/empty-state";
import { Tab, TabList, TabPanel, Tabs } from "@/shared/ui/tabs";
import { FissuresPanel } from "./FissuresPanel";
import { type Detail, NowColumn } from "./NowColumn";
import { TraderPanel } from "./TraderPanel";

/** Age in seconds after which the world state counts as stale. */
const STALE_SECS = 90;

const NO_NODES = new Map();

/** Timers page: every world timer on the left, the selected detail panel on the right. */
export function TimersPage() {
  const t = useT();
  const now = useNow();
  const state = useAppState();
  const world = state?.worldstate ?? null;
  const [detail, setDetail] = useKept<Detail>("timers.detail", "fissures");
  const nodeKeys = world
    ? [
        ...new Set([
          ...world.fissures.map((each) => each.node),
          ...world.visits.map((each) => each.node),
        ]),
      ]
    : [];
  const nodes = useNodes(nodeKeys).data ?? NO_NODES;
  if (!world) return <EmptyState title={t("timers-waiting")} />;

  const shelf = (vendor: string) =>
    state?.shelves.find((each) => each.vendor === vendor)?.offers ?? [];
  const openFissures = world.fissures.filter(
    (fissure) => fissure.expires > now && !fissure.storm,
  ).length;

  return (
    <div className="flex min-w-0 flex-1">
      <aside className="w-72 shrink-0 overflow-y-auto border-hairline border-r px-2 py-4">
        <NowColumn
          cycles={world.cycles}
          deadlines={world.deadlines}
          visits={world.visits}
          nodes={nodes}
          now={now}
          detail={detail}
          onDetail={setDetail}
        />
        {now - world.at > STALE_SECS && (
          <div className="mt-4 px-2.5 text-small text-warn">
            {t("timers-stale", { minutes: Math.floor((now - world.at) / MINUTE) })}
          </div>
        )}
      </aside>
      <Tabs
        value={detail}
        onValueChange={(value) => setDetail(value as Detail)}
        className="flex min-w-0 flex-1 flex-col"
      >
        <TabList className="shrink-0 px-6 pt-3">
          <Tab value="fissures" count={openFissures}>
            {t("timers-fissures")}
          </Tab>
          {world.visits.map((visit) => (
            <Tab
              key={visit.vendor}
              value={visit.vendor}
              count={shelf(visit.vendor).length || undefined}
            >
              {t("trader-name", { vendor: visit.vendor })}
            </Tab>
          ))}
          <Tab value="syndicates">{t("syndicates-title")}</Tab>
        </TabList>
        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
          <TabPanel value="fissures">
            <FissuresPanel fissures={world.fissures} nodes={nodes} now={now} />
          </TabPanel>
          {world.visits.map((visit) => (
            <TabPanel key={visit.vendor} value={visit.vendor}>
              <TraderPanel
                visit={visit}
                offers={shelf(visit.vendor)}
                node={nodes.get(visit.node)}
                now={now}
              />
            </TabPanel>
          ))}
          <TabPanel value="syndicates">
            <SyndicatesPanel />
          </TabPanel>
        </div>
      </Tabs>
    </div>
  );
}
