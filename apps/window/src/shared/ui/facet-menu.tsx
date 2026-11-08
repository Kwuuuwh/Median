import type { MessageId } from "@median/i18n";
import { Check, ChevronDown, Minus } from "lucide-react";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";
import { RangeField } from "./range-field";

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

/** Lower and upper bound of one value a list can be filtered by, as typed; empty leaves a side open. */
export type FacetRange = {
  label: string;
  from: string;
  to: string;
  /** Whether each side holds text that is not an allowed number. */
  invalidFrom: boolean;
  invalidTo: boolean;
  onChange: (from: string, to: string) => void;
};

/** Short word for a range that is set: both bounds, or the one side typed; null while open. */
export function rangeWord(range: FacetRange, fromHint: string, toHint: string): string | null {
  if (range.from === "" && range.to === "") return null;
  if (range.to === "") return `${range.label} ${fromHint} ${range.from}`;
  if (range.from === "") return `${range.label} ${toHint} ${range.to}`;
  return `${range.label} ${range.from}–${range.to}`;
}

/**
 * Button opening a list's filters: yes/no filters, each click cycling one from unset to yes, to no
 * and back to unset, then value ranges typed as numbers. The button names the filters that are set.
 */
export function FacetMenu<K extends Facet>({
  facets,
  values,
  onChange,
  ranges = [],
  onClear,
  className,
}: {
  facets: readonly K[];
  values: ReadonlyMap<K, boolean>;
  onChange: (values: ReadonlyMap<K, boolean>) => void;
  ranges?: readonly FacetRange[];
  /** Clears every filter at once; without it the yes/no filters and each range clear in turn. */
  onClear?: () => void;
  className?: string;
}) {
  const t = useT();
  const label = t("facet-title");
  const fromHint = t("range-from");
  const toHint = t("range-to");
  const choices = facets.map((key) => ({ key, yes: t(WORDS[key][0]), no: t(WORDS[key][1]) }));
  const words = [
    ...choices.flatMap((choice) => {
      const value = values.get(choice.key);
      return value === undefined ? [] : [value ? choice.yes : choice.no];
    }),
    ...ranges.flatMap((range) => rangeWord(range, fromHint, toHint) ?? []),
  ];
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
  const clear = () => {
    if (onClear) {
      onClear();
      return;
    }
    onChange(new Map());
    for (const range of ranges) range.onChange("", "");
  };
  return (
    <Popover>
      <PopoverTrigger
        aria-label={label}
        className={cn(
          "inline-flex h-8 items-center justify-between gap-1.5 rounded-control border bg-raised px-2.5 text-body outline-none hover:border-dim hover:text-ink",
          words.length > 0 ? "border-accent/60 text-ink" : "border-line text-soft",
          className,
        )}
      >
        <span className="truncate">{shown}</span>
        <ChevronDown className="size-4 shrink-0 text-dim" />
      </PopoverTrigger>
      <PopoverContent className="min-w-(--radix-popover-trigger-width) p-1">
        {choices.map((choice) => {
          const value = values.get(choice.key);
          return (
            <button
              type="button"
              key={choice.key}
              onClick={() => step(choice.key)}
              className={cn(
                "flex h-8 w-full cursor-default items-center gap-2 rounded-control px-2 text-body outline-none hover:bg-surface focus-visible:bg-surface",
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
            </button>
          );
        })}
        {ranges.length > 0 && (
          <div
            className={cn(
              "grid gap-2 px-2 py-2",
              choices.length > 0 && "mt-1 border-hairline border-t",
            )}
          >
            {ranges.map((range) => (
              <RangeField
                key={range.label}
                label={range.label}
                from={range.from}
                to={range.to}
                onFrom={(from) => range.onChange(from, range.to)}
                onTo={(to) => range.onChange(range.from, to)}
                fromHint={fromHint}
                toHint={toHint}
                invalidFrom={range.invalidFrom}
                invalidTo={range.invalidTo}
                className="whitespace-nowrap"
              />
            ))}
          </div>
        )}
        {words.length > 0 && (
          <>
            <div className="my-1 h-px bg-hairline" />
            <button
              type="button"
              onClick={clear}
              className="flex h-8 w-full cursor-default items-center rounded-control px-2 text-body text-muted outline-none hover:bg-surface hover:text-ink focus-visible:bg-surface"
            >
              {t("fissure-reset")}
            </button>
          </>
        )}
      </PopoverContent>
    </Popover>
  );
}
