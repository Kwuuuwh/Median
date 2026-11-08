import {
  CommandError,
  commands,
  events,
  type Release,
  type UpdateProgressed,
  unwrap,
} from "@median/ipc";
import { useSyncExternalStore } from "react";

/** Where the app update stands. */
export type UpdateStep =
  | { step: "idle" }
  | { step: "checking" }
  | { step: "latest" }
  | { step: "found"; release: Release }
  | { step: "downloading"; release: Release; downloaded: number; total: number | null }
  | { step: "installing"; release: Release }
  | { step: "failed"; during: "check" | "install"; reason: string };

let current: UpdateStep = { step: "idle" };
let checking: Promise<UpdateStep> | null = null;
const listeners = new Set<() => void>();

function publish(next: UpdateStep): UpdateStep {
  current = next;
  for (const listener of listeners) listener();
  return next;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function reasonOf(error: unknown): string {
  return error instanceof CommandError ? String(error.reason) : String(error);
}

/** The step after `progress` arrives while `release` downloads. */
export function progressed(release: Release, progress: UpdateProgressed): UpdateStep {
  const { downloaded, total } = progress;
  if (total !== null && downloaded >= total) return { step: "installing", release };
  return { step: "downloading", release, downloaded, total };
}

/** Share of the download received, from 0 to 1; null while the size is unknown. */
export function downloadedShare(step: UpdateStep): number | null {
  if (step.step !== "downloading" || step.total === null || step.total === 0) return null;
  return Math.min(step.downloaded / step.total, 1);
}

/** Checks the chosen channel for a release; joins a check already under way. */
export function checkUpdate(): Promise<UpdateStep> {
  if (checking) return checking;
  publish({ step: "checking" });
  checking = unwrap(commands.checkUpdate())
    .then((release) => publish(release ? { step: "found", release } : { step: "latest" }))
    .catch((error: unknown) =>
      publish({ step: "failed", during: "check", reason: reasonOf(error) }),
    )
    .finally(() => {
      checking = null;
    });
  return checking;
}

/** Downloads and installs `release`, the one the last check found. */
export async function installUpdate(release: Release): Promise<void> {
  publish({ step: "downloading", release, downloaded: 0, total: null });
  const unlisten = await events.updateProgressed.listen(({ payload }) => {
    publish(progressed(release, payload));
  });
  try {
    await unwrap(commands.installUpdate());
  } catch (error) {
    publish({ step: "failed", during: "install", reason: reasonOf(error) });
  } finally {
    unlisten();
  }
}

/** Checks again once any check under way ends, so a channel just chosen is the one checked. */
export async function recheckUpdate(): Promise<UpdateStep> {
  await checking;
  forgetUpdate();
  return checkUpdate();
}

/** Forgets what the last check found, unless it is downloading or installing. */
export function forgetUpdate(): void {
  if (current.step === "downloading" || current.step === "installing") return;
  publish({ step: "idle" });
}

/** Where the app update stands, re-rendering the caller on change. */
export function useUpdateStep(): UpdateStep {
  return useSyncExternalStore(subscribe, () => current);
}
