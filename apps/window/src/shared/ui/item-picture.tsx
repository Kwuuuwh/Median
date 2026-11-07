import { ImageOff } from "lucide-react";
import { useState } from "react";
import { useAppState } from "@/shared/data/app-state";
import { cn } from "@/shared/lib/cn";
import { picture } from "@/shared/lib/picture";

/** An item's picture from the game data, or a placeholder icon when it is missing or fails. */
export function ItemPicture({ image, className }: { image: string | null; className?: string }) {
  const catalog = useAppState()?.catalog.installed ?? null;
  const [broken, setBroken] = useState(false);
  if (!image || broken) {
    return (
      <div className={cn("flex items-center justify-center text-dim/50", className)}>
        <ImageOff className="size-1/3" />
      </div>
    );
  }
  return (
    <img
      src={picture(image, catalog)}
      alt=""
      draggable={false}
      loading="lazy"
      onError={() => setBroken(true)}
      className={cn("object-contain", className)}
    />
  );
}
