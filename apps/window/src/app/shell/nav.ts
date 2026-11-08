import type { MessageId } from "@median/i18n";
import type { ReleaseChannel } from "@median/ipc";
import {
  ArrowLeftRight,
  Backpack,
  Bot,
  Gem,
  GraduationCap,
  Library,
  type LucideIcon,
  Orbit,
  Sparkles,
} from "lucide-react";

/** One sidebar link to a page. */
type NavEntry = {
  to: string;
  label: MessageId;
  icon: LucideIcon;
  /** Channels whose builds show the link; every channel when absent. */
  shownIn?: readonly ReleaseChannel[];
};

/** A labelled group of pages. */
export type NavGroup = { label: MessageId; entries: readonly NavEntry[] };

/** Sidebar groups and links, top to bottom. */
export const NAV: readonly NavGroup[] = [
  {
    label: "nav-world",
    entries: [
      { to: "/timers", label: "timers-title", icon: Orbit },
      { to: "/catalog", label: "catalogue-title", icon: Library, shownIn: ["preview", "dev"] },
      { to: "/mastery", label: "mastery-title", icon: GraduationCap },
    ],
  },
  {
    label: "nav-trading",
    entries: [
      { to: "/inventory", label: "inventory-title", icon: Backpack },
      { to: "/orders", label: "orders-title", icon: ArrowLeftRight },
      { to: "/agent", label: "agent-name", icon: Bot },
      { to: "/relics", label: "relics-title", icon: Gem },
      { to: "/rivens", label: "rivens-title", icon: Sparkles, shownIn: ["dev"] },
    ],
  },
];

/** Whether a build of `channel` shows the link; a limited link waits until the channel is known. */
export function shows(entry: NavEntry, channel: ReleaseChannel | undefined): boolean {
  if (!entry.shownIn) return true;
  return channel !== undefined && entry.shownIn.includes(channel);
}
