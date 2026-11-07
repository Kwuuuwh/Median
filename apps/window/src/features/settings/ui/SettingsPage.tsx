import type { MessageId } from "@median/i18n";
import { type LucideIcon, Palette, RefreshCw, SlidersHorizontal } from "lucide-react";
import { useAppState } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { useKept } from "@/shared/lib/use-kept";
import { AppearanceSettings } from "./AppearanceSettings";
import { GeneralSettings } from "./GeneralSettings";
import { UpdateSettings } from "./UpdateSettings";

type Group = "general" | "appearance" | "updates";

/** Settings groups, top to bottom. */
const GROUPS: readonly { value: Group; label: MessageId; note: MessageId; icon: LucideIcon }[] = [
  {
    value: "general",
    label: "settings-general",
    note: "settings-general-note",
    icon: SlidersHorizontal,
  },
  {
    value: "appearance",
    label: "settings-appearance",
    note: "settings-appearance-note",
    icon: Palette,
  },
  { value: "updates", label: "settings-updates", note: "settings-updates-note", icon: RefreshCw },
];

/** Settings page: the groups on the left, the chosen group's settings on the right. */
export function SettingsPage() {
  const t = useT();
  const state = useAppState();
  const [group, setGroup] = useKept<Group>("settings.group", "general");
  if (!state) return null;
  const shown = GROUPS.find((each) => each.value === group) ?? GROUPS[0];

  return (
    <div className="flex min-w-0 flex-1">
      <nav className="w-60 shrink-0 space-y-0.5 border-hairline border-r p-3">
        {GROUPS.map((each) => (
          <button
            key={each.value}
            type="button"
            onClick={() => setGroup(each.value)}
            className={cn(
              "flex w-full items-start gap-2.5 rounded-control px-2.5 py-2 text-left transition-colors hover:bg-raised",
              each.value === shown.value && "bg-raised",
            )}
          >
            <each.icon
              className={cn(
                "mt-0.5 size-4 shrink-0 text-muted",
                each.value === shown.value && "text-accent",
              )}
            />
            <span className="grid min-w-0 gap-0.5">
              <span className="text-body text-ink">{t(each.label)}</span>
              <span className="text-caption text-dim">{t(each.note)}</span>
            </span>
          </button>
        ))}
      </nav>
      <section className="min-w-0 flex-1 overflow-y-auto px-8 py-6">
        <div className="max-w-2xl space-y-6">
          <h1 className="font-display font-semibold text-heading text-ink">{t(shown.label)}</h1>
          {shown.value === "general" && <GeneralSettings state={state} />}
          {shown.value === "appearance" && <AppearanceSettings accent={state.accent} />}
          {shown.value === "updates" && <UpdateSettings state={state} />}
        </div>
      </section>
    </div>
  );
}
