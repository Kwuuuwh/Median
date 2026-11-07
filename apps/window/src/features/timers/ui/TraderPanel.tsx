import type { Node, Offer, Visit } from "@median/ipc";
import { useT } from "@/shared/i18n";
import { BaroShelf } from "./BaroShelf";
import { useRemaining } from "./remaining";
import { VarziaShelf } from "./VarziaShelf";

/** A vendor's panel: its offers during a visit, otherwise the time until arrival and the relay. */
export function TraderPanel({
  visit,
  offers,
  node,
  now,
}: {
  visit: Visit;
  offers: readonly Offer[];
  node: Node | undefined;
  now: number;
}) {
  const t = useT();
  const remaining = useRemaining();
  const here = visit.arrives <= now && now < visit.departs;
  if (!here) {
    return (
      <div className="py-8">
        <div className="text-muted text-small first-letter:uppercase">
          {t("trader-until-arrival")}
        </div>
        <div className="mt-1 font-semibold text-figure text-ink tabular-nums">
          {remaining(visit.arrives - now)}
        </div>
        <div className="mt-1 text-body text-soft">
          {node ? `${node.name}, ${node.planet}` : visit.node}
        </div>
      </div>
    );
  }
  return visit.vendor === "baro" ? <BaroShelf offers={offers} /> : <VarziaShelf offers={offers} />;
}
