import { ACCENTS, commands, send } from "@median/ipc";
import { useT } from "@/shared/i18n";
import { accentPaint } from "@/shared/lib/accent";
import { cn } from "@/shared/lib/cn";
import { SettingSection } from "./SettingRow";

/** Accent colour of the interface, one swatch per accent the backend accepts. */
export function AppearanceSettings({ accent }: { accent: string }) {
  const t = useT();
  return (
    <SettingSection title={t("settings-accent")}>
      <div className="grid grid-cols-3 gap-2 py-3">
        {ACCENTS.map((code) => (
          <button
            key={code}
            type="button"
            aria-pressed={code === accent}
            onClick={() => send(commands.setAccent(code))}
            className={cn(
              "flex items-center gap-2.5 rounded-control border border-line px-3 py-2 text-body text-soft transition-colors hover:border-dim hover:text-ink",
              code === accent && "border-accent text-ink",
            )}
          >
            <span
              className="size-4 shrink-0 rounded-full"
              style={{ backgroundColor: accentPaint(code) }}
            />
            {t("accent-name", { accent: code })}
          </button>
        ))}
      </div>
    </SettingSection>
  );
}
