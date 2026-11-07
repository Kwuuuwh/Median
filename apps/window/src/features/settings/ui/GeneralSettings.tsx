import { LOCALES, translate } from "@median/i18n";
import { type AppState, commands, send } from "@median/ipc";
import { useEffect, useState } from "react";
import { useT } from "@/shared/i18n";
import { Segmented } from "@/shared/ui/segmented";
import { Slider } from "@/shared/ui/slider";
import { Switch } from "@/shared/ui/switch";
import { SettingRow, SettingSection } from "./SettingRow";

/** Interface language, and the sound of in-game whispers with its volume. */
export function GeneralSettings({ state }: { state: AppState }) {
  const t = useT();
  const { alert } = state;
  // The slider moves freely while dragged; the backend hears only the value it is released at.
  const [volume, setVolume] = useState(alert.volume);
  useEffect(() => setVolume(alert.volume), [alert.volume]);

  return (
    <>
      <SettingSection title={t("settings-language")}>
        <SettingRow label={t("settings-interface-language")} hint={t("settings-language-hint")}>
          <Segmented
            label={t("settings-language")}
            value={state.locale}
            segments={LOCALES.map((code) => ({
              value: code,
              label: translate(code, "language-name"),
            }))}
            onChange={(code) => send(commands.setLocale(code))}
          />
        </SettingRow>
      </SettingSection>
      <SettingSection title={t("settings-notifications")}>
        <SettingRow label={t("settings-whisper-sound")} hint={t("settings-whisper-sound-hint")}>
          <Switch
            aria-label={t("settings-whisper-sound")}
            checked={alert.whisper}
            onCheckedChange={(whisper) => send(commands.setAlert({ ...alert, whisper }))}
          />
        </SettingRow>
        <SettingRow label={t("alert-volume")}>
          <Slider
            aria-label={t("alert-volume")}
            className="w-48"
            min={0}
            max={100}
            step={5}
            value={[volume]}
            disabled={!alert.whisper}
            onValueChange={([next]) => setVolume(next)}
            onValueCommit={([next]) => send(commands.setAlert({ ...alert, volume: next }))}
          />
          <span className="w-10 text-right text-muted text-small tabular-nums">{volume}%</span>
        </SettingRow>
      </SettingSection>
    </>
  );
}
