import type { Build, ReleaseChannel } from "@median/ipc";
import { type Translate, useT } from "@/shared/i18n";
import { releaseName } from "@/shared/lib/revision";
import { Button } from "@/shared/ui/button";
import { checkUpdate, downloadedShare, installUpdate, type UpdateStep } from "../model/updater";

/** The check for a release of the chosen channel, or the install of the one found. */
export function UpdateAction({ step }: { step: UpdateStep }) {
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

/**
 * One line on where the update stands; with nothing newer in a channel other than the build's
 * own, the wait for that channel to pass this build.
 */
export function updateStatus(
  t: Translate,
  step: UpdateStep,
  chosen: ReleaseChannel,
  build: Build,
): string {
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
      return t("update-found", { revision: releaseName(step.release) });
    case "downloading": {
      const share = downloadedShare(step);
      const revision = releaseName(step.release);
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
