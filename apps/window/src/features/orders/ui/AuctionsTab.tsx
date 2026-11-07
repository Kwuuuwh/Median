import type { Good, OwnAuction, Riven } from "@median/ipc";
import { RefreshCw } from "lucide-react";
import { useState } from "react";
import { RivenDialog } from "@/features/rivens/ui/RivenDialog";
import { useAppState } from "@/shared/data/app-state";
import { useGoods } from "@/shared/data/catalog";
import { useOwnAuctions, useRivenAppraisals, useRivenNames } from "@/shared/data/rivens";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/button";
import { EmptyState } from "@/shared/ui/empty-state";
import { TileGrid } from "@/shared/ui/tile-grid";
import { AuctionTile } from "./AuctionTile";

/** An auction whose riven stats are known. */
type ReadAuction = OwnAuction & { riven: Riven };

/** The account's riven auctions as tiles; requires sign-in. */
export function AuctionsTab() {
  const t = useT();
  const signedIn = useAppState()?.membership.standing === "in";
  const auctions = useOwnAuctions();
  const own = auctions.data ?? [];
  const read = own.filter((auction): auction is ReadAuction => auction.riven !== null);
  const weapons = [
    ...new Set(own.flatMap((auction) => (auction.weapon ? [auction.weapon] : []))),
  ].sort();
  const goods = new Map((useGoods(weapons).data ?? []).map((good: Good) => [good.item, good]));
  const auctionNames = useRivenNames(read.map((auction) => auction.riven)).data ?? [];
  const auctionAppraisals = useRivenAppraisals(read);
  const [openedAuction, setOpenedAuction] = useState<string | null>(null);

  const nameOf = new Map(read.map((auction, at) => [auction.id, auctionNames[at]]));
  const appraisalOf = new Map(read.map((auction, at) => [auction.id, auctionAppraisals[at]]));
  const auction = read.find((each) => each.id === openedAuction) ?? null;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-line border-b px-6 py-3">
        <Button
          tone="ghost"
          size="icon"
          aria-label={t("trade-refresh")}
          className="ml-auto"
          disabled={!signedIn || auctions.isFetching}
          onClick={() => void auctions.refetch()}
        >
          <RefreshCw className={auctions.isFetching ? "animate-spin" : undefined} />
        </Button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-4 pb-16">
        {!signedIn ? (
          <EmptyState title={t("sell-signed-out")} />
        ) : auctions.isPending ? (
          <EmptyState title={t("deal-orders-asking")} />
        ) : own.length === 0 ? (
          <EmptyState title={t("auctions-none")} />
        ) : (
          <TileGrid least={320} className="items-start">
            {own.map((each) => (
              <AuctionTile
                key={each.id}
                auction={each}
                weapon={each.weapon ? goods.get(each.weapon) : undefined}
                name={nameOf.get(each.id)}
                appraisal={appraisalOf.get(each.id)}
                onOpen={() => setOpenedAuction(each.id)}
              />
            ))}
          </TileGrid>
        )}
      </div>
      <RivenDialog
        opened={auction && { key: auction.id, riven: auction.riven, rank: auction.rank }}
        weapon={auction ? goods.get(auction.riven.weapon) : undefined}
        name={auction && (nameOf.get(auction.id) ?? auction.name)}
        appraisal={auction && appraisalOf.get(auction.id)}
        onClose={() => setOpenedAuction(null)}
      />
    </div>
  );
}
