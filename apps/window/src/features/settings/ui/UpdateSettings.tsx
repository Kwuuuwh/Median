import {
  type AppState,
  type Build,
  type Catalog,
  commands,
  type ReleaseChannel,
  send,
} from "@median/ipc";
import {
  checkUpdate,
  downloadedShare,
  forgetUpdate,
  installUpdate,
  type UpdateStep,
  useUpdateStep,
} from "@/features/update/model/updater";
import { ChangeList } from "@/features/update/ui/ChangeList";
import { useBuild, useLocale } from "@/shared/data/app-state";
import { useGameData } from "@/shared/data/catalog";
import { type Translate, useT } from "@/shared/i18n";
import { dataProgress } from "@/shared/lib/data-progress";
import { formatDate } from "@/shared/lib/format";
import { revisionText } from "@/shared/lib/revision";
import { Button } from "@/shared/ui/button";
import { Segmented } from "@/shared/ui/segmented";
import { SettingRow, SettingSection } from "./SettingRow";

/** Channels a released build can take updates from. */
const CHANNELS: readonly ReleaseChannel[] = ["stable", "preview"];

/** The settings' updates group: build, channel, new release, game data and changes. */
export function UpdateSettings({ state }: { state: AppState }) {
  const t = useT();
  const locale = useLocale();
  const build = useBuild();
  const step = useUpdateStep();
  if (!build) return null;
  const busy = step.step === "downloading" || step.step === "installing";

  return (
    <>
      <SettingSection title={t("settings-build")}>
        <SettingRow label={t("settings-build-version")}>
          <span className="text-soft tabular-nums">{revisionText(build, locale)}</span>
        </SettingRow>
        {build.updates ? (
          <SettingRow label={t("settings-channel")} hint={t("settings-channel-hint")}>
            <Segmented
              label={t("settings-channel")}
              value={state.updateChannel ?? build.channel}
              segments={CHANNELS.map((channel) => ({
                value: channel,
                label: t("release-channel", { channel }),
              }))}
              onChange={(channel) => {
                if (busy) return;
                forgetUpdate();
                send(commands.setUpdateChannel(channel));
              }}
            />
          </SettingRow>
        ) : (
          <SettingRow label={t("settings-channel")} hint={t("settings-channel-local")}>
            <span className="text-soft">{t("release-channel", { channel: build.channel })}</span>
          </SettingRow>
        )}
        <SettingRow label={t("settings-log")} hint={t("settings-log-hint")}>
          <Button size="sm" onClick={() => send(commands.showLogFolder())}>
            {t("settings-log-open")}
          </Button>
        </SettingRow>
      </SettingSection>
      {build.updates && <NewRelease step={step} />}
      <GameDataSection catalog={state.catalog} />
      <SettingSection title={t("settings-changes")}>
        {build.changes.length > 0 ? (
          <ChangeList changes={build.changes} className="py-3" />
        ) : (
          <div className="py-3 text-dim text-small">{t(emptyChanges(build))}</div>
        )}
      </SettingSection>
    </>
  );
}

/** The text shown when the build lists no changes. */
function emptyChanges(build: Build) {
  return build.channel === "dev" ? "settings-changes-local" : "settings-changes-none";
}

/** The check for a release of the chosen channel and, once found, its install. */
function NewRelease({ step }: { step: UpdateStep }) {
  const t = useT();
  return (
    <SettingSection title={t("settings-update")}>
      <SettingRow label={status(t, step)}>
        {(step.step === "idle" || step.step === "latest" || step.step === "failed") && (
          <Button size="sm" onClick={() => void checkUpdate()}>
            {t("update-check")}
          </Button>
        )}
        {step.step === "found" && (
          <Button size="sm" tone="primary" onClick={() => void installUpdate(step.release)}>
            {t("update-install")}
          </Button>
        )}
      </SettingRow>
      {step.step === "found" && step.release.changes.length > 0 && (
        <ChangeList
          changes={step.release.changes}
          className="max-h-48 border-hairline border-t py-3"
        />
      )}
    </SettingSection>
  );
}

/** Game data states in which a check is under way. */
const DATA_BUSY: ReadonlySet<Catalog["activity"]["state"]> = new Set([
  "checking",
  "fetching",
  "opening",
]);

/** The game data's version, date and game build, with a check for newer data. */
function GameDataSection({ catalog }: { catalog: Catalog }) {
  const t = useT();
  const locale = useLocale();
  const data = useGameData().data;
  const busy = DATA_BUSY.has(catalog.activity.state);
  const progress = dataProgress(t, locale, catalog);
  const hint =
    catalog.activity.state === "idle"
      ? data?.gameBuild && t("settings-data-game", { build: data.gameBuild })
      : progress.title;
  return (
    <SettingSection title={t("settings-data")}>
      <SettingRow label={t("settings-build-version")} hint={hint || undefined}>
        {data && (
          <span className="text-soft tabular-nums">
            {data.fetched === null
              ? data.version
              : `${data.version}, ${t("settings-data-date", { date: formatDate(locale, data.fetched) })}`}
          </span>
        )}
        <Button size="sm" disabled={busy} onClick={() => send(commands.syncCatalog())}>
          {t("update-check")}
        </Button>
      </SettingRow>
    </SettingSection>
  );
}

/** One line on where the update stands. */
function status(t: Translate, step: UpdateStep): string {
  switch (step.step) {
    case "idle":
      return t("update-unchecked");
    case "checking":
      return t("update-checking");
    case "latest":
      return t("update-latest");
    case "found":
      return t("update-found", { revision: `r${step.release.revision}` });
    case "downloading": {
      const share = downloadedShare(step);
      const revision = `r${step.release.revision}`;
      return share === null
        ? t("update-downloading", { revision })
        : t("update-downloading-share", { revision, percent: Math.floor(share * 100) });
    }
    case "installing":
      return t("update-installing");
    case "failed":
      return t(step.during === "check" ? "update-check-failed" : "update-install-failed", {
        reason: step.reason,
      });
  }
}
