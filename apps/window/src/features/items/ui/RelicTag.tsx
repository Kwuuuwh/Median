import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Tooltip } from "@/shared/ui/tooltip";
import { relicLabel } from "../model/relic";
import { RARITY_TEXT } from "./rarity";

/** A relic's short name, coloured by the item's reward rarity in it. */
export function RelicTag({ name, rarity }: { name: string; rarity: string }) {
  const t = useT();
  return (
    <Tooltip hint={t("rarity-name", { rarity })}>
      <span
        className={cn(
          "whitespace-nowrap rounded-sm bg-raised px-1.5 text-caption",
          RARITY_TEXT[rarity] ?? "text-muted",
        )}
      >
        {relicLabel(name)}
      </span>
    </Tooltip>
  );
}
