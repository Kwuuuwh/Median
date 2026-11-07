import type { Policy } from "@median/ipc";
import { type ReactNode, useState } from "react";
import { useSetPolicy } from "@/shared/data/trading";
import { useT } from "@/shared/i18n";
import { MOST_COPIES } from "@/shared/lib/order";
import { Button } from "@/shared/ui/button";
import { NumberField, wholeNumber } from "@/shared/ui/number-field";
import { Segmented } from "@/shared/ui/segmented";
import { Switch } from "@/shared/ui/switch";

/** Budget source: the user's fixed limit, or a share of the platinum held. */
type Budget = "own" | "purse";

/** The policy fields the panel edits, numbers kept as typed text. */
type Draft = {
  budget: Budget;
  mostTied: string;
  purseShare: string;
  leastProfit: string;
  leastExchanges: string;
  mostPerOrder: string;
  rankUp: boolean;
  rivens: boolean;
  rivenCapital: string;
  rivenShare: string;
  rivenMargin: string;
  rivenLeastProfit: string;
};

function draftOf(policy: Policy): Draft {
  return {
    budget: policy.tiedFromPurse ? "purse" : "own",
    mostTied: String(policy.mostTied),
    purseShare: String(policy.purseShare),
    leastProfit: String(policy.leastProfit),
    leastExchanges: String(policy.leastExchanges),
    mostPerOrder: String(policy.mostPerOrder),
    rankUp: policy.rankUp,
    rivens: policy.rivens,
    rivenCapital: String(policy.rivenCapital),
    rivenShare: String(policy.rivenShare),
    rivenMargin: String(policy.rivenMargin),
    rivenLeastProfit: String(policy.rivenLeastProfit),
  };
}

/** Percent of the platinum held that the item budget leaves for rivens: 100 minus `purseShare`. */
function rivenRoom(draft: Draft): number {
  return 100 - (wholeNumber(draft.purseShare, 1, 100) ?? 100);
}

/** Allowed inclusive range of each numeric draft field. */
function bounds(
  draft: Draft,
): Record<Exclude<keyof Draft, "budget" | "rankUp" | "rivens">, [number, number]> {
  return {
    mostTied: [0, 1_000_000],
    purseShare: [1, 100],
    leastProfit: [1, 10_000],
    leastExchanges: [0, 100],
    mostPerOrder: [1, MOST_COPIES],
    rivenCapital: [0, 1_000_000],
    rivenShare: [0, rivenRoom(draft)],
    rivenMargin: [1, 90],
    rivenLeastProfit: [0, 100_000],
  };
}

/** The policy built from the draft; null while any field holds no number in its range. */
function policyOf(policy: Policy, draft: Draft): Policy | null {
  const read = Object.fromEntries(
    Object.entries(bounds(draft)).map(([field, [low, high]]) => [
      field,
      wholeNumber(draft[field as keyof Draft] as string, low, high),
    ]),
  );
  if (Object.values(read).some((value) => value === null)) return null;
  return {
    ...policy,
    tiedFromPurse: draft.budget === "purse",
    mostTied: read.mostTied as number,
    purseShare: read.purseShare as number,
    leastProfit: read.leastProfit as number,
    leastExchanges: read.leastExchanges as number,
    mostPerOrder: read.mostPerOrder as number,
    rankUp: draft.rankUp,
    rivens: draft.rivens,
    rivenCapital: read.rivenCapital as number,
    rivenShare: read.rivenShare as number,
    rivenMargin: read.rivenMargin as number,
    rivenLeastProfit: read.rivenLeastProfit as number,
  };
}

/** One settings section under its title, with an optional control at the title's right end. */
function Part({
  title,
  aside,
  children,
}: {
  title: string;
  aside?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <section className="grid gap-2 border-hairline border-t pt-3 first:border-t-0 first:pt-0">
      <header className="flex h-6 items-center justify-between">
        <span className="font-medium text-body text-ink">{title}</span>
        {aside}
      </header>
      {children}
    </section>
  );
}

/** Agent policy editor: the platinum it may commit, and its item and riven trading limits. */
export function AgentSettings({ policy }: { policy: Policy }) {
  const t = useT();
  const save = useSetPolicy();
  const [draft, setDraft] = useState<Draft>(() => draftOf(policy));
  const next = policyOf(policy, draft);
  const changed = next !== null && JSON.stringify(next) !== JSON.stringify(policy);
  const range = bounds(draft);
  const field = (name: keyof typeof range, label: string) => (
    <NumberField
      label={label}
      value={draft[name]}
      onChange={(value) => setDraft({ ...draft, [name]: value })}
      invalid={wholeNumber(draft[name], ...range[name]) === null}
      min={range[name][0]}
      max={range[name][1]}
      width="w-20"
      className="justify-between"
    />
  );

  return (
    <div className="grid w-72 gap-3">
      <Part title={t("policy-capital")}>
        <Segmented<Budget>
          label={t("policy-budget")}
          className="justify-self-start"
          value={draft.budget}
          onChange={(budget) => setDraft({ ...draft, budget })}
          segments={[
            { value: "own", label: t("policy-budget-own") },
            { value: "purse", label: t("policy-budget-purse") },
          ]}
        />
        {draft.budget === "own" ? (
          <>
            {field("mostTied", t("policy-items"))}
            {draft.rivens && field("rivenCapital", t("policy-rivens-amount"))}
          </>
        ) : (
          <>
            {field("purseShare", t("policy-items-share"))}
            {draft.rivens && field("rivenShare", t("policy-rivens-share"))}
          </>
        )}
      </Part>
      <Part title={t("policy-section-items")}>
        {field("leastProfit", t("policy-a-trade"))}
        {field("mostPerOrder", t("policy-most-per-order"))}
        {field("leastExchanges", t("policy-least-exchanges"))}
      </Part>
      <Part
        title={t("policy-rank-up")}
        aside={
          <Switch
            aria-label={t("policy-rank-up")}
            checked={draft.rankUp}
            onCheckedChange={(rankUp) => setDraft({ ...draft, rankUp })}
          />
        }
      />
      <Part
        title={t("policy-rivens")}
        aside={
          <Switch
            aria-label={t("policy-rivens")}
            checked={draft.rivens}
            onCheckedChange={(rivens) => setDraft({ ...draft, rivens })}
          />
        }
      >
        {draft.rivens && (
          <>
            {field("rivenMargin", t("policy-riven-margin"))}
            {field("rivenLeastProfit", t("policy-riven-least-profit"))}
          </>
        )}
      </Part>
      <Button
        tone="primary"
        disabled={!changed || save.isPending}
        onClick={() => next && save.mutate(next)}
        className="justify-self-end"
      >
        {t("orders-edit-save")}
      </Button>
    </div>
  );
}
