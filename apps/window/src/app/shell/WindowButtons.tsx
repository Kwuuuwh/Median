import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";
import { useT } from "@/shared/i18n";

const BUTTON =
  "inline-flex h-10 w-11 items-center justify-center text-muted transition-colors hover:bg-raised hover:text-ink [&_svg]:size-4";

/**
 * Minimise, maximise and close, for a window drawn without the system frame; without the maximise
 * button for a window that cannot be resized.
 */
export function WindowButtons({ maximize = true }: { maximize?: boolean }) {
  const t = useT();
  const window = getCurrentWindow();
  return (
    <div className="flex">
      <button
        type="button"
        className={BUTTON}
        aria-label={t("titlebar-minimize")}
        onClick={() => void window.minimize()}
      >
        <Minus />
      </button>
      {maximize && (
        <button
          type="button"
          className={BUTTON}
          aria-label={t("titlebar-maximize")}
          onClick={() => void window.toggleMaximize()}
        >
          <Square className="size-3.5!" />
        </button>
      )}
      <button
        type="button"
        className={`${BUTTON} hover:bg-danger hover:text-canvas`}
        aria-label={t("titlebar-close")}
        onClick={() => void window.close()}
      >
        <X />
      </button>
    </div>
  );
}
