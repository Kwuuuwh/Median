import { Outlet, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";
import { PickListener } from "@/features/agent/ui/PickListener";
import { AlarmListener } from "@/features/alarms/ui/AlarmListener";
import { ItemDialog } from "@/features/items/ui/card/ItemDialog";
import { useAppState } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { accentPaint } from "@/shared/lib/accent";
import { Toaster } from "@/shared/ui/toast";
import { TooltipProvider } from "@/shared/ui/tooltip";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";

/** `MouseEvent.button` value of the mouse "back" side button. */
const MOUSE_BACK = 3;

/** Title bar, sidebar and page. */
export function AppShell() {
  const state = useAppState();
  const t = useT();
  const router = useRouter();

  useEffect(() => {
    function back(event: MouseEvent): void {
      if (event.button !== MOUSE_BACK) return;
      event.preventDefault();
      router.history.back();
    }
    window.addEventListener("mouseup", back);
    return () => window.removeEventListener("mouseup", back);
  }, [router]);

  useEffect(() => {
    if (!state) return;
    document.documentElement.style.setProperty("--paint-accent", accentPaint(state.accent));
  }, [state]);

  if (!state) return null;
  return (
    <TooltipProvider delayDuration={400}>
      <div className="flex h-full flex-col">
        <TitleBar state={state} />
        <div className="flex min-h-0 flex-1">
          <Sidebar />
          <main className="flex min-w-0 flex-1">
            <Outlet />
          </main>
        </div>
      </div>
      <AlarmListener />
      <PickListener />
      <ItemDialog />
      <Toaster dismiss={t("toast-dismiss")} />
    </TooltipProvider>
  );
}
