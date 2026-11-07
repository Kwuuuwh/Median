import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { ItemPicture } from "@/shared/ui/item-picture";
import { Tooltip } from "@/shared/ui/tooltip";

/** An item's framed picture: green ring when mastered, fainter accent ring when crafted. */
export function ItemThumb({
  image,
  mastered = false,
  crafted = false,
  size = "md",
}: {
  image: string | null;
  mastered?: boolean;
  /** Whether the item this one builds is held. */
  crafted?: boolean;
  size?: "sm" | "md";
}) {
  const t = useT();
  const thumb = (
    <ItemPicture
      image={image}
      className={cn(
        "shrink-0 rounded-control bg-sunken",
        size === "md" ? "size-14 p-1" : "size-7 p-0.5",
        mastered ? "ring-1 ring-live/60" : crafted && "ring-1 ring-accent/50",
      )}
    />
  );
  return mastered || crafted ? (
    <Tooltip hint={t(mastered ? "item-mastered" : "item-crafted")}>
      <span className="inline-flex shrink-0">{thumb}</span>
    </Tooltip>
  ) : (
    thumb
  );
}
