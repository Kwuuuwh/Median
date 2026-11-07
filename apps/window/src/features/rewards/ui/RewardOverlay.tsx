import { LoaderCircle } from "lucide-react";
import { useAppState } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { row } from "../model/row";
import { RewardCard } from "./RewardCard";

/**
 * Overlay over the game's reward screen with the value of each reward card, or a loading notice
 * while the cards are read; renders nothing while the reward screen is closed.
 */
export function RewardOverlay() {
  const t = useT();
  const rewards = useAppState()?.game.rewards;
  if (!rewards || rewards.state === "gone") return null;
  return (
    <div className="pointer-events-none fixed inset-0">
      <div className="absolute inset-0 m-auto aspect-video max-h-full max-w-full">
        {rewards.state === "reading" ? (
          <div className="absolute top-[53%] left-1/2 flex -translate-x-1/2 items-center gap-2 rounded-xl border border-white/10 bg-black/80 px-4 py-2.5 text-sm text-soft backdrop-blur-sm">
            <LoaderCircle className="size-4 animate-spin" />
            {t("overlay-updating")}
          </div>
        ) : (
          <div
            className="absolute top-[53%] grid gap-2 rounded-xl border border-white/10 bg-black/80 p-2 backdrop-blur-sm"
            style={row(rewards.cards.length)}
          >
            {rewards.cards.map((card, at) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: cards are keyed by screen position
              <RewardCard key={at} card={card} fresh={rewards.fresh} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
