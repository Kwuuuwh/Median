import { formatBytes } from "@median/i18n";
import { type ReactNode, useEffect } from "react";
import {
  checkUpdate,
  downloadedShare,
  installUpdate,
  useUpdateStep,
} from "@/features/update/model/updater";
import { ChangeList } from "@/features/update/ui/ChangeList";
import { useLocale } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/button";
import { GateBar } from "./GateBar";

/**
 * The launcher's update step: the check, the release found, its download and install. `onSettled`
 * must keep its identity across renders.
 */
export function UpdateGate({ onSettled }: { onSettled: () => void }) {
  const t = useT();
  const locale = useLocale();
  const step = useUpdateStep();

  useEffect(() => {
    void checkUpdate().then((next) => {
      if (next.step === "latest" || next.step === "failed") onSettled();
    });
  }, [onSettled]);

  switch (step.step) {
    case "idle":
    case "checking":
      return (
        <Centered title={t("update-checking")}>
          <Button tone="ghost" size="sm" onClick={onSettled}>
            {t("update-skip")}
          </Button>
        </Centered>
      );
    case "found":
      return (
        <div className="flex min-h-0 flex-1 flex-col gap-3 py-4">
          <div className="font-display font-semibold text-heading">
            {t("update-found", { revision: `r${step.release.revision}` })}
          </div>
          <ChangeList changes={step.release.changes} className="min-h-0 flex-1" />
          <div className="flex justify-end gap-2">
            <Button tone="ghost" onClick={onSettled}>
              {t("update-later")}
            </Button>
            <Button tone="primary" onClick={() => void installUpdate(step.release)}>
              {t("update-install")}
            </Button>
          </div>
        </div>
      );
    case "downloading": {
      const share = downloadedShare(step);
      return (
        <Centered title={t("update-downloading", { revision: `r${step.release.revision}` })}>
          <div className="text-muted tabular-nums">
            {step.total === null
              ? formatBytes(locale, step.downloaded)
              : `${formatBytes(locale, step.downloaded)} / ${formatBytes(locale, step.total)}`}
          </div>
          {share !== null && <GateBar share={share} />}
        </Centered>
      );
    }
    case "installing":
      return (
        <Centered title={t("update-installing")}>
          <div className="text-muted">{t("update-installing-hint")}</div>
        </Centered>
      );
    case "failed":
      return (
        <Centered title={t("update-failed")}>
          <div className="max-w-full truncate text-muted" title={step.reason}>
            {step.reason}
          </div>
          <Button onClick={onSettled}>{t("update-continue")}</Button>
        </Centered>
      );
    case "latest":
      return null;
  }
}

/** A launcher step's title over its detail, in the middle of the window. */
function Centered({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex min-w-0 flex-1 flex-col items-center justify-center gap-3">
      <div className="font-display font-semibold text-heading">{title}</div>
      {children}
    </div>
  );
}
