import type { Way } from "@median/ipc";
import {
  CalendarClock,
  CircleHelp,
  FlaskConical,
  Gem,
  type LucideIcon,
  MapPin,
  Skull,
  Store,
  Users,
} from "lucide-react";
import type { ReactNode } from "react";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { formatTenths } from "@/shared/lib/format";
import { Money } from "@/shared/ui/money";
import { Tooltip } from "@/shared/ui/tooltip";
import { levelsOf, placeText } from "../model/place";
import { VendorCost } from "./VendorCost";

/** Icon of each acquisition way. */
function iconOf(way: Way | null): LucideIcon {
  switch (way?.kind) {
    case "credits":
      return Store;
    case "vendor":
      return Users;
    case "lab":
      return FlaskConical;
    case "relics":
      return Gem;
    case "drop":
      return way.enemy ? Skull : MapPin;
    case "visit":
      return CalendarClock;
    default:
      return CircleHelp;
  }
}

/** One line on how an item is obtained: the vendor or place, and its cost there; `compact` moves
 * the cost into a tooltip. */
export function WayLine({
  way,
  compact = false,
  className,
}: {
  way: Way | null;
  compact?: boolean;
  className?: string;
}) {
  const t = useT();
  const locale = useLocale();
  const Icon = iconOf(way);
  let label: string;
  let detail: ReactNode = null;
  switch (way?.kind) {
    case "credits":
      label = way.vendor;
      detail = <Money currency="credits" amount={way.credits} />;
      break;
    case "vendor":
      label = way.vendor;
      detail = (
        <>
          {way.rank !== null && <span>{t("card-rank", { rank: way.rank })}</span>}
          {way.cost !== null && (
            <VendorCost cost={way.cost} currency={way.currency} coin={way.coin} />
          )}
        </>
      );
      break;
    case "lab":
      label = way.lab;
      break;
    case "relics":
      label = t("mastery-way-relics");
      detail = way.vaulted ? <span className="text-duc">{t("relic-vaulted")}</span> : null;
      break;
    case "drop": {
      label = placeText(way);
      const levels = levelsOf(way);
      detail = (
        <>
          {levels && <span className="tabular-nums">{levels}</span>}
          <span className="tabular-nums">{formatTenths(locale, way.chance)} %</span>
        </>
      );
      break;
    }
    case "visit":
      label = way.vendor;
      break;
    default:
      label = t("mastery-way-unknown");
  }
  const costs = detail && (
    <span className="flex shrink-0 items-center gap-1.5 text-muted [&>*+*]:before:mr-1.5 [&>*+*]:before:content-['·']">
      {detail}
    </span>
  );
  const line = (
    <span
      className={cn(
        "flex min-w-0 items-center gap-1.5 text-small",
        way ? "text-soft" : "text-dim",
        className,
      )}
    >
      <span className="flex min-w-0 items-center gap-1.5">
        <Icon className="size-3.5 shrink-0 text-muted" />
        <span className="truncate">{label}</span>
      </span>
      {!compact && costs}
    </span>
  );
  if (!compact || !costs) return line;
  return (
    <Tooltip
      hint={
        <span className="grid gap-1">
          {label}
          {costs}
        </span>
      }
    >
      {line}
    </Tooltip>
  );
}
