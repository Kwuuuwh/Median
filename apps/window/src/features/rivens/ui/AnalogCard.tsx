import type { RivenAnalog } from "@median/ipc";
import { useT } from "@/shared/i18n";
import { Money } from "@/shared/ui/money";
import { RIVEN_TOP_RANK } from "../model/held";
import { SellerMark } from "./SellerMark";
import { StatText } from "./StatText";

/**
 * One similar riven auction: price, seller, rank and rerolls, and its listed stats; stats the
 * compared riven lacks are struck through.
 */
export function AnalogCard({ analog, held }: { analog: RivenAnalog; held: ReadonlySet<string> }) {
  const t = useT();
  return (
    <article className="flex flex-col gap-2 rounded-panel border border-line bg-raised/40 p-2.5">
      <header className="flex items-center gap-2">
        <Money
          currency="platinum"
          amount={analog.price}
          className="font-semibold text-ink text-title"
        />
        <span className="flex-1" />
        {analog.seller && <SellerMark seller={analog.seller} status={analog.status ?? null} />}
      </header>
      <span className="text-caption text-dim tabular-nums">
        {t("inventory-rank", { rank: analog.rank, max: RIVEN_TOP_RANK })} ·{" "}
        {t("riven-rolls-count", { count: analog.rerolls })}
      </span>
      <div className="grid gap-0.5 text-small">
        {analog.stats.map((stat) => (
          <StatText
            key={`${stat.tag}${stat.isCurse}`}
            stat={stat}
            isCurse={stat.isCurse}
            isStruck={!held.has(`${stat.tag}${stat.isCurse}`)}
          />
        ))}
      </div>
    </article>
  );
}
