import type { Watch } from "@median/ipc";
import { Bell, BellRing, Check } from "lucide-react";
import { useWatches, useWatchToggle, watchKey } from "@/shared/data/watches";
import { cn } from "@/shared/lib/cn";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";

const BELL =
  "inline-flex size-6 shrink-0 items-center justify-center rounded-sm transition-colors hover:bg-raised [&_svg]:size-3.5";

function useIsWatched(): (watch: Watch) => boolean {
  const watched = new Set(useWatches().map(watchKey));
  return (watch) => watched.has(watchKey(watch));
}

/** Bell button that adds or removes one watch. */
export function WatchBell({ watch, label }: { watch: Watch; label: string }) {
  const isWatched = useIsWatched();
  const toggle = useWatchToggle();
  const on = isWatched(watch);
  return (
    <button
      type="button"
      aria-pressed={on}
      aria-label={label}
      title={label}
      onClick={(event) => {
        event.stopPropagation();
        toggle.mutate({ watch, on: !on });
      }}
      className={cn(BELL, on ? "text-accent" : "text-dim hover:text-ink")}
    >
      {on ? <BellRing /> : <Bell />}
    </button>
  );
}

/** One entry of a bell menu. */
export type WatchChoice = { watch: Watch; label: string };

/** Bell that opens a menu of watches for one subject; each entry adds or removes its own watch. */
export function WatchMenu({ choices, label }: { choices: readonly WatchChoice[]; label: string }) {
  const isWatched = useIsWatched();
  const toggle = useWatchToggle();
  const any = choices.some((choice) => isWatched(choice.watch));
  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-label={label}
          title={label}
          onClick={(event) => event.stopPropagation()}
          className={cn(BELL, any ? "text-accent" : "text-dim hover:text-ink")}
        >
          {any ? <BellRing /> : <Bell />}
        </button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-52 p-1" onClick={(event) => event.stopPropagation()}>
        <ul>
          {choices.map((choice) => {
            const on = isWatched(choice.watch);
            return (
              <li key={watchKey(choice.watch)}>
                <button
                  type="button"
                  aria-pressed={on}
                  onClick={() => toggle.mutate({ watch: choice.watch, on: !on })}
                  className="flex h-8 w-full items-center gap-2 rounded-control px-2 text-left text-body text-soft hover:bg-surface"
                >
                  <span className="flex size-4 items-center justify-center text-accent">
                    {on && <Check className="size-3.5" />}
                  </span>
                  <span className="first-letter:uppercase">{choice.label}</span>
                </button>
              </li>
            );
          })}
        </ul>
      </PopoverContent>
    </Popover>
  );
}
