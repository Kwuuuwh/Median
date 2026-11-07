import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { followEvents } from "@/app/events";
import { queryClient } from "@/app/query";
import { scaleOf } from "@/features/rewards/model/stage";
import { RewardOverlay } from "@/features/rewards/ui/RewardOverlay";
import { CycleOverlay } from "@/features/rivens/ui/CycleOverlay";
import { TooltipProvider } from "@/shared/ui/tooltip";
import "./styles.css";

/** Root font size at 1920×1080, the layout size of the overlay; all other sizes are in rem. */
const ROOT_PX = 16;

/** Scales the root font size to the game's screen by the same rule the game scales its UI. */
function scaleRoot(): void {
  document.documentElement.style.fontSize = `${ROOT_PX * scaleOf(innerWidth, innerHeight)}px`;
}

async function start(): Promise<void> {
  if (import.meta.env.MODE === "mock") await import("@/mock/overlay");

  const root = document.getElementById("root");
  if (!root) throw new Error("overlay.html has no #root");

  scaleRoot();
  addEventListener("resize", scaleRoot);
  await followEvents(queryClient);
  createRoot(root).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <RewardOverlay />
          <CycleOverlay />
        </TooltipProvider>
      </QueryClientProvider>
    </StrictMode>,
  );
}

void start();
