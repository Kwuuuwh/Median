import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

/** Whether the system permits notifications; the permission is requested once per run. */
let granted: Promise<boolean> | null = null;

async function permitted(): Promise<boolean> {
  granted ??= isPermissionGranted().then(
    async (already) => already || (await requestPermission()) === "granted",
  );
  return granted;
}

/**
 * Shows a system notification while the window is out of focus and permission is granted; does
 * nothing while the window has focus, where it shows its own notice.
 */
export async function notifySystem(title: string, body?: string): Promise<void> {
  if (document.hasFocus()) return;
  if (!(await permitted())) return;
  sendNotification({ title, body });
}
