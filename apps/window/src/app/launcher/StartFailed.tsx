import { type Build, commands, type StartFailure, send } from "@median/ipc";
import { TriangleAlert } from "lucide-react";
import { useEffect } from "react";
import { checkUpdate, useUpdateStep } from "@/features/update/model/updater";
import { UpdateAction, updateStatus } from "@/features/update/ui/UpdateStatus";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/button";

/** A start that failed: why, and the update of the build's own channel that may fix it. */
export function StartFailed({ failure, build }: { failure: StartFailure; build: Build }) {
  const t = useT();
  const step = useUpdateStep();

  useEffect(() => {
    if (build.updates) void checkUpdate();
  }, [build.updates]);

  return (
    <div className="flex min-h-0 flex-1 flex-col justify-center gap-3">
      <div className="flex items-center gap-2 font-display font-semibold text-heading">
        <TriangleAlert className="size-5 shrink-0 text-danger" />
        {t(failure.newerData ? "start-newer-data" : "start-failed")}
      </div>
      {!failure.newerData && (
        <div className="wrap-anywhere line-clamp-3 text-muted text-small">{failure.reason}</div>
      )}
      {build.updates && (
        <div className="flex items-center justify-between gap-3">
          <span className="min-w-0 text-body text-soft">
            {updateStatus(t, step, build.channel, build)}
          </span>
          <UpdateAction step={step} />
        </div>
      )}
      <Button
        size="sm"
        tone="ghost"
        className="self-start"
        onClick={() => send(commands.showLogFolder())}
      >
        {t("settings-log-open")}
      </Button>
    </div>
  );
}
