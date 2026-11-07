import { X } from "lucide-react";
import { Toast } from "radix-ui";
import { useSyncExternalStore } from "react";
import { cn } from "@/shared/lib/cn";

/** A toast in the window corner: a failed command, or a triggered watch. */
export type Notice = { tone: "failure" | "alarm"; title: string; detail?: string };

type Shown = Notice & { id: number };

let shown: Shown[] = [];
let nextId = 0;
const listeners = new Set<() => void>();

function publish(next: Shown[]): void {
  shown = next;
  for (const listener of listeners) listener();
}

/** Shows a notice in the corner of the window. */
export function showNotice(notice: Notice): void {
  publish([...shown, { ...notice, id: nextId++ }]);
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Renders queued notices in the window corner; each closes after 8 s. */
export function Toaster({ dismiss }: { dismiss: string }) {
  const notices = useSyncExternalStore(subscribe, () => shown);
  return (
    <Toast.Provider duration={8000}>
      {notices.map((notice) => (
        <Toast.Root
          key={notice.id}
          onOpenChange={(open) => !open && publish(shown.filter((each) => each.id !== notice.id))}
          className={cn(
            "data-[state=closed]:fade-out-0 data-[state=open]:slide-in-from-right-4 grid w-80 grid-cols-[1fr_auto] gap-x-2 rounded-panel border bg-raised p-3 shadow-black/40 shadow-xl data-[state=closed]:animate-out data-[state=open]:animate-in",
            notice.tone === "failure" ? "border-danger/40" : "border-accent/40",
          )}
        >
          <Toast.Title
            className={cn(
              "wrap-anywhere min-w-0 font-medium text-body",
              notice.tone === "failure" ? "text-danger" : "text-ink",
            )}
          >
            {notice.title}
          </Toast.Title>
          <Toast.Close aria-label={dismiss} className="text-dim hover:text-ink">
            <X className="size-4" />
          </Toast.Close>
          {notice.detail && (
            <Toast.Description className="wrap-anywhere col-span-2 mt-1 min-w-0 select-text text-small text-soft">
              {notice.detail}
            </Toast.Description>
          )}
        </Toast.Root>
      ))}
      <Toast.Viewport className="fixed right-4 bottom-4 z-50 flex flex-col gap-2 outline-none" />
    </Toast.Provider>
  );
}
