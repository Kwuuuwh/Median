import {
  type AppState,
  type Build,
  type Catalog,
  commands,
  type ReleaseChannel,
  send,
} from "@median/ipc";
import { useEffect, useRef } from "react";
import {
  checkUpdate,
  downloadedShare,
  installUpdate,
  recheckUpdate,
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

/** The settings' updates group: build with its update, channel, game data and changes. */
export function UpdateSettings({ state }: { state: AppState }) {
  const build = useBuild();
  if (!build) return null;
  return <Updates state={state} build={build} />;
}

function Updates({ state, build }: { state: AppState; build: Build }) {
  const t = useT();
  const locale = useLocale();
  const step = useUpdateStep();
  const busy = step.step === "downloading" || step.step === "installing";
  const chosen = state.updateChannel ?? build.channel;
  const checked = useRef(chosen);

  useEffect(() => {
    if (checked.current === chosen) return;
    checked.current = chosen;
    if (build.updates) void recheckUpdate();
  }, [chosen, build.updates]);

  return (
    <>
      <SettingSection title={t("settings-build")}>
        <SettingRow
          label={t("settings-build-version")}
          hint={build.updates ? status(t, step, chosen, build) : undefined}
        >
          <span className="text-soft tabular-nums">{revisionText(build, locale)}</span>
          {build.updates && <UpdateAction step={step} />}
        </SettingRow>
        {step.step === "found" && step.release.changes.length > 0 && (
          <ChangeList
            changes={step.release.changes}
            className="max-h-48 border-hairline border-b py-3"
          />
        )}
        {build.updates ? (
          <SettingRow label={t("settings-channel")} hint={t("settings-channel-hint")}>
            <Segmented
              label={t("settings-channel")}
              value={chosen}
              segments={CHANNELS.map((channel) => ({
                value: channel,
                label: t("release-channel", { channel }),
              }))}
              onChange={(channel) => {
                if (!busy) send(commands.setUpdateChannel(channel));
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

/** The check for a release of the chosen channel, or the install of the one found. */
function UpdateAction({ step }: { step: UpdateStep }) {
  const t = useT();
  if (step.step === "found") {
    return (
      <Button size="sm" tone="primary" onClick={() => void installUpdate(step.release)}>
        {t("update-install")}
      </Button>
    );
  }
  if (step.step === "idle" || step.step === "latest" || step.step === "failed") {
    return (
      <Button size="sm" onClick={() => void checkUpdate()}>
        {t("update-check")}
      </Button>
    );
  }
  return null;
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

/**
 * One line on where the update stands; with nothing newer in a channel other than the build's
 * own, the wait for that channel to pass this build.
 */
function status(t: Translate, step: UpdateStep, chosen: ReleaseChannel, build: Build): string {
  switch (step.step) {
    case "idle":
      return t("update-unchecked");
    case "checking":
      return t("update-checking");
    case "latest":
      return chosen === build.channel
        ? t("update-latest")
        : t("update-waiting", { channel: chosen, revision: `r${build.revision}` });
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
