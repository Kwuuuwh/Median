import { useState } from "react";
import { useSyndicates } from "@/shared/data/syndicates";
import { useT } from "@/shared/i18n";
import { EmptyState } from "@/shared/ui/empty-state";
import { TileGrid } from "@/shared/ui/tile-grid";
import { SyndicateShelf } from "./SyndicateShelf";
import { SyndicateTile } from "./SyndicateTile";

/** Every syndicate as a tile, or the opened syndicate's offers; an empty state until loaded. */
export function SyndicatesPanel() {
  const t = useT();
  const syndicates = useSyndicates().data;
  const [opened, setOpened] = useState<string | null>(null);
  if (!syndicates) return <EmptyState title={t("syndicates-unread")} />;

  const shelf = syndicates.find((syndicate) => syndicate.tag === opened);
  if (shelf) return <SyndicateShelf syndicate={shelf} onBack={() => setOpened(null)} />;

  return (
    <TileGrid least={272}>
      {syndicates.map((syndicate) => (
        <SyndicateTile
          key={syndicate.tag}
          syndicate={syndicate}
          onOpen={() => setOpened(syndicate.tag)}
        />
      ))}
    </TileGrid>
  );
}
