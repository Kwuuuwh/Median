import { Star } from "lucide-react";
import { useFavourites, useFavouriteToggle } from "@/shared/data/favourites";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";

/** Star button that toggles an item's favourite mark. */
export function FavouriteStar({
  item,
  className,
  quiet = false,
}: {
  item: string;
  className?: string;
  /** Whether an unmarked star is visible only while its card is hovered or it has focus. */
  quiet?: boolean;
}) {
  const t = useT();
  const on = useFavourites().data?.has(item) ?? false;
  const toggle = useFavouriteToggle();
  return (
    <button
      type="button"
      aria-label={t("item-favourite")}
      aria-pressed={on}
      onClick={() => toggle.mutate({ item, on: !on })}
      className={cn(
        "inline-flex size-7 shrink-0 items-center justify-center rounded-control transition-colors hover:bg-raised",
        on ? "text-accent" : "text-dim hover:text-soft",
        quiet && !on && "opacity-0 focus-visible:opacity-100 group-hover:opacity-100",
        className,
      )}
    >
      <Star className={cn("size-4", on && "fill-current")} />
    </button>
  );
}
