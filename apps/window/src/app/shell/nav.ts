import type { MessageId } from "@median/i18n";
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
  /** Shown only in dev builds. */
  dev?: boolean;
};

/** A labelled group of pages. */
export type NavGroup = { label: MessageId; entries: readonly NavEntry[] };

/** Sidebar groups and links, top to bottom. */
export const NAV: readonly NavGroup[] = [
  {
    label: "nav-world",
    entries: [
      { to: "/timers", label: "timers-title", icon: Orbit },
      { to: "/catalog", label: "catalogue-title", icon: Library },
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
      { to: "/rivens", label: "rivens-title", icon: Sparkles, dev: true },
    ],
  },
];
