import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { formatCount } from "@/shared/lib/format";
import type { VeiledCount } from "../model/held";

/** Tile with the number of veiled rivens the account holds, per class and in total. */
export function VeiledTile({ veiled }: { veiled: readonly VeiledCount[] }) {
  const t = useT();
  const locale = useLocale();
  const total = veiled.reduce((sum, each) => sum + each.count, 0);
  return (
    <article className="flex flex-col gap-2.5 rounded-panel border border-line bg-surface p-3">
      <header className="flex items-baseline gap-2">
        <span className="flex-1 font-medium text-body text-soft">{t("rivens-veiled")}</span>
        <span className="font-semibold text-ink text-title tabular-nums">
          {formatCount(locale, total)}
        </span>
      </header>
      <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-small">
        {veiled.map((each) => (
          <span key={each.class} className="flex items-baseline gap-2">
            <span className="flex-1 truncate text-muted">
              {t("riven-class", { class: each.class })}
            </span>
            <span className="text-soft tabular-nums">{formatCount(locale, each.count)}</span>
          </span>
        ))}
      </div>
    </article>
  );
}
