import { PackageOpen } from "lucide-react";
import type { ReactNode } from "react";
import { useT } from "@/shared/i18n";
import { FavouriteStar } from "./FavouriteStar";
import { HeldMark } from "./HeldMark";
import { ItemThumb } from "./ItemThumb";

/** Minimum item tile width in pixels; grids of item tiles pass it to `TileGrid`. */
export const ITEM_TILE_WIDTH = 288;

/** Item tile: picture, name with held count, a `meta` line under the name, and a body below. */
export function ItemTile({
  item,
  image,
  name,
  held,
  mastered,
  crafted = false,
  known = true,
  meta,
  onOpen,
  children,
}: {
  item: string;
  image: string | null;
  name: string;
  held: number;
  mastered: boolean;
  /** Whether the item this one builds is held. */
  crafted?: boolean;
  /** Whether the item is in the catalog; an unknown one gets a placeholder, no star, no card. */
  known?: boolean;
  /** Content under the name, such as the price. */
  meta?: ReactNode;
  /** Opens the item's card; the whole tile is clickable except controls with their own action. */
  onOpen?: () => void;
  children?: ReactNode;
}) {
  const t = useT();
  return (
    <article className="relative flex flex-col rounded-panel border border-line bg-surface transition-colors hover:border-dim">
      <header className="flex gap-3 p-3">
        {known ? (
          <ItemThumb image={image} mastered={mastered} crafted={crafted} />
        ) : (
          <span className="flex size-14 shrink-0 items-center justify-center rounded-control border border-line border-dashed text-dim">
            <PackageOpen className="size-6" />
          </span>
        )}
        <div className="grid min-w-0 flex-1 content-start gap-1.5 pt-0.5">
          <h3 className="wrap-anywhere flex flex-wrap items-center gap-x-2 gap-y-1 font-medium text-body text-ink">
            {onOpen && known ? (
              <button
                type="button"
                onClick={onOpen}
                className="text-left after:absolute after:inset-0 after:rounded-panel"
              >
                {name}
              </button>
            ) : (
              name
            )}
            <HeldMark count={held} />
          </h3>
          {!known && <span className="text-dim text-small">{t("shelf-unknown")}</span>}
          {meta}
        </div>
        {known && (
          <span className="relative z-10">
            <FavouriteStar item={item} />
          </span>
        )}
      </header>
      {children && (
        <div className="relative z-10 mt-auto border-hairline border-t px-3 pt-2 pb-2.5">
          {children}
        </div>
      )}
    </article>
  );
}
