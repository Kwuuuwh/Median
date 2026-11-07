import type { MessageId } from "@median/i18n";
import { Check, ChevronDown, Minus } from "lucide-react";
import { DropdownMenu } from "radix-ui";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";

/** Every yes/no filter a list in the window can be filtered by. */
export type Facet =
  | "held"
  | "listed"
  | "tradable"
  | "favourite"
  | "prime"
  | "vaulted"
  | "built"
  | "many"
  | "equipped"
  | "upgraded"
  | "spare"
  | "mastered"
  | "planned"
  | "reachable"
  | "ready"
  | "relics"
  | "opened"
  | "affordable"
  | "beaten"
  | "visible"
  | "agent";

/** Message ids of each facet's labels: for yes, then for no. */
const WORDS: Record<Facet, readonly [MessageId, MessageId]> = {
  held: ["facet-held", "facet-unheld"],
  listed: ["facet-listed", "facet-unlisted"],
  tradable: ["facet-tradable", "facet-untradable"],
  favourite: ["facet-favourite", "facet-unfavoured"],
  prime: ["facet-prime", "facet-plain"],
  vaulted: ["facet-vaulted", "facet-unvaulted"],
  built: ["facet-built", "facet-unbuilt"],
  many: ["facet-many", "facet-single"],
  equipped: ["facet-equipped", "facet-free"],
  upgraded: ["facet-upgraded", "facet-unranked"],
  spare: ["facet-spare", "facet-unspared"],
  mastered: ["facet-mastered", "facet-unmastered"],
  planned: ["facet-planned", "facet-unplanned"],
  reachable: ["facet-reachable", "facet-unreachable"],
  ready: ["facet-ready", "facet-unready"],
  relics: ["facet-relics", "facet-no-relics"],
  opened: ["facet-opened", "facet-closed"],
  affordable: ["facet-affordable", "facet-unaffordable"],
  beaten: ["facet-beaten", "facet-best"],
  visible: ["facet-visible", "facet-hidden"],
  agent: ["facet-agent", "facet-mine"],
};

/**
 * Dropdown of yes/no filters for a list; each click cycles a filter from unset to yes, to no, and
 * back to unset.
 */
export function FacetMenu<K extends Facet>({
  facets,
  values,
  onChange,
  className,
}: {
  facets: readonly K[];
  values: ReadonlyMap<K, boolean>;
  onChange: (values: ReadonlyMap<K, boolean>) => void;
  className?: string;
}) {
  const t = useT();
  const label = t("facet-title");
  const choices = facets.map((key) => ({ key, yes: t(WORDS[key][0]), no: t(WORDS[key][1]) }));
  const words = choices.flatMap((choice) => {
    const value = values.get(choice.key);
    return value === undefined ? [] : [value ? choice.yes : choice.no];
  });
  const shown =
    words.length === 0 ? label : words.length === 1 ? words[0] : `${words[0]} +${words.length - 1}`;
  const step = (key: K) => {
    const next = new Map(values);
    const value = next.get(key);
    if (value === undefined) next.set(key, true);
    else if (value) next.set(key, false);
    else next.delete(key);
    onChange(next);
  };
  return (
    <DropdownMenu.Root modal={false}>
      <DropdownMenu.Trigger
        aria-label={label}
        className={cn(
          "inline-flex h-8 items-center justify-between gap-1.5 rounded-control border bg-raised px-2.5 text-body outline-none hover:border-dim hover:text-ink",
          words.length > 0 ? "border-accent/60 text-ink" : "border-line text-soft",
          className,
        )}
      >
        <span className="truncate">{shown}</span>
        <ChevronDown className="size-4 shrink-0 text-dim" />
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="start"
          sideOffset={6}
          className="z-50 min-w-(--radix-dropdown-menu-trigger-width) rounded-panel border border-line bg-raised p-1 shadow-black/40 shadow-xl"
        >
          {choices.map((choice) => {
            const value = values.get(choice.key);
            return (
              <DropdownMenu.Item
                key={choice.key}
                onSelect={(event) => {
                  event.preventDefault();
                  step(choice.key);
                }}
                className={cn(
                  "flex h-8 cursor-default items-center gap-2 rounded-control px-2 text-body outline-none data-[highlighted]:bg-surface",
                  value === undefined ? "text-soft" : "text-ink",
                )}
              >
                <span
                  className={cn(
                    "flex size-4 items-center justify-center rounded-sm border",
                    value === undefined
                      ? "border-line"
                      : value
                        ? "border-accent text-accent"
                        : "border-danger/70 text-danger",
                  )}
                >
                  {value === true && <Check className="size-3" />}
                  {value === false && <Minus className="size-3" />}
                </span>
                {value === false ? choice.no : choice.yes}
              </DropdownMenu.Item>
            );
          })}
          {values.size > 0 && (
            <>
              <DropdownMenu.Separator className="my-1 h-px bg-hairline" />
              <DropdownMenu.Item
                onSelect={() => onChange(new Map())}
                className="flex h-8 cursor-default items-center rounded-control px-2 text-body text-muted outline-none data-[highlighted]:bg-surface data-[highlighted]:text-ink"
              >
                {t("fissure-reset")}
              </DropdownMenu.Item>
            </>
          )}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
