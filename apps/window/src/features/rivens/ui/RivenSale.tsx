import type { Riven, RivenBand } from "@median/ipc";
import { Check, Tag } from "lucide-react";
import { useState } from "react";
import { useAppState } from "@/shared/data/app-state";
import { usePutUpRiven } from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/button";
import { NumberField, wholeNumber } from "@/shared/ui/number-field";
import { Segmented } from "@/shared/ui/segmented";
import { Tooltip } from "@/shared/ui/tooltip";

/** Sale type: a fixed price, or an auction with bids from a starting price. */
type SaleKind = "fixed" | "auction";

const MOST_PLATINUM = 100_000;

/** A price field's text: the typed text, else `asked` rounded, else empty. */
function priceText(typed: string | null, asked: number | null | undefined): string {
  return typed ?? (asked == null ? "" : String(Math.round(asked)));
}

/** Form that lists a riven at a fixed price or as an auction, prefilled from similar prices. */
export function RivenSale({
  riven,
  rank,
  band,
}: {
  riven: Riven;
  rank: number;
  band: RivenBand | null | undefined;
}) {
  const t = useT();
  const signedIn = useAppState()?.membership.standing === "in";
  const [kind, setKind] = useState<SaleKind>("fixed");
  const [fixedTyped, setFixed] = useState<string | null>(null);
  const [startingTyped, setStarting] = useState<string | null>(null);
  const [buyoutTyped, setBuyout] = useState<string | null>(null);
  const putUp = usePutUpRiven();

  const fixed = priceText(fixedTyped, band?.middle);
  const starting = priceText(startingTyped, band?.low);
  const buyout = priceText(buyoutTyped, band?.middle);
  const fixedPrice = wholeNumber(fixed, 1, MOST_PLATINUM);
  const startingPrice = wholeNumber(starting, 1, MOST_PLATINUM);
  const buyoutPrice = wholeNumber(buyout, startingPrice ?? 1, MOST_PLATINUM);
  const sale =
    kind === "fixed"
      ? fixedPrice !== null && { starting: fixedPrice, buyout: fixedPrice }
      : startingPrice !== null &&
        (buyout === "" || buyoutPrice !== null) && { starting: startingPrice, buyout: buyoutPrice };
  const edit = (set: (value: string) => void) => (value: string) => {
    set(value);
    putUp.reset();
  };

  const button = (
    <Button
      size="sm"
      className="w-32 shrink-0"
      disabled={!signedIn || !sale || putUp.isPending || putUp.isSuccess}
      onClick={() => sale && putUp.mutate({ riven, rank, ...sale })}
    >
      {putUp.isSuccess ? <Check /> : <Tag />}
      {t(putUp.isSuccess ? "sell-placed" : "sell-place")}
    </Button>
  );
  return (
    <div className="flex items-center gap-3">
      <Segmented<SaleKind>
        label={t("riven-sale-kind")}
        value={kind}
        onChange={(next) => {
          setKind(next);
          putUp.reset();
        }}
        segments={[
          { value: "fixed", label: t("riven-sale-fixed") },
          { value: "auction", label: t("riven-sale-auction") },
        ]}
      />
      <div className="flex min-w-0 flex-1 items-center gap-3">
        {kind === "fixed" ? (
          <NumberField
            label={t("sell-price")}
            value={fixed}
            onChange={edit(setFixed)}
            invalid={fixed !== "" && fixedPrice === null}
            width="w-20"
          />
        ) : (
          <>
            <NumberField
              label={t("riven-starting")}
              value={starting}
              onChange={edit(setStarting)}
              invalid={starting !== "" && startingPrice === null}
              width="w-20"
            />
            <NumberField
              label={t("riven-buyout")}
              value={buyout}
              onChange={edit(setBuyout)}
              invalid={buyout !== "" && buyoutPrice === null}
              width="w-20"
            />
          </>
        )}
      </div>
      {signedIn ? (
        button
      ) : (
        <Tooltip hint={t("sell-signed-out")}>
          <span className="shrink-0">{button}</span>
        </Tooltip>
      )}
    </div>
  );
}
