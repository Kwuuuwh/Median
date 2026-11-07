import { Link } from "@tanstack/react-router";
import { type LucideIcon, PanelLeftClose, PanelLeftOpen, Settings } from "lucide-react";
import { useState } from "react";
import { useBuild } from "@/shared/data/app-state";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Tooltip } from "@/shared/ui/tooltip";
import { NAV } from "./nav";

const COLLAPSED_KEY = "median.sidebar.collapsed";

function readCollapsed(): boolean {
  try {
    return localStorage.getItem(COLLAPSED_KEY) === "1";
  } catch {
    return false;
  }
}

function writeCollapsed(collapsed: boolean): void {
  try {
    localStorage.setItem(COLLAPSED_KEY, collapsed ? "1" : "0");
  } catch {
    // Without storage the setting lasts only until the window closes.
  }
}

/** Page navigation in labelled groups; collapses to icons, and the choice is saved locally. */
export function Sidebar() {
  const t = useT();
  const [collapsed, setCollapsed] = useState(readCollapsed);
  const dev = useBuild()?.channel === "dev";

  function toggle(): void {
    setCollapsed(!collapsed);
    writeCollapsed(!collapsed);
  }

  return (
    <nav
      className={cn(
        "flex shrink-0 flex-col border-hairline border-r bg-surface py-3 transition-[width] duration-150",
        collapsed ? "w-14" : "w-52",
      )}
    >
      <div className="flex-1 space-y-4 overflow-y-auto overflow-x-hidden px-2">
        {NAV.map((group) => (
          <div key={group.label}>
            <div
              className={cn(
                "mb-1 h-5 truncate px-2.5 text-caption text-dim",
                collapsed && "invisible",
              )}
            >
              {t(group.label)}
            </div>
            <ul className="space-y-0.5">
              {group.entries
                .filter((entry) => dev || !entry.dev)
                .map((entry) => (
                  <li key={entry.to}>
                    <NavLink
                      to={entry.to}
                      label={t(entry.label)}
                      icon={entry.icon}
                      collapsed={collapsed}
                    />
                  </li>
                ))}
            </ul>
          </div>
        ))}
      </div>
      <div className="space-y-0.5 px-2">
        <NavLink to="/settings" label={t("settings-title")} icon={Settings} collapsed={collapsed} />
        <button
          type="button"
          onClick={toggle}
          aria-label={t(collapsed ? "nav-expand" : "nav-collapse")}
          className="flex h-8 w-full items-center gap-2.5 rounded-control px-2.5 text-dim transition-colors hover:bg-raised hover:text-ink"
        >
          {collapsed ? <PanelLeftOpen className="size-4" /> : <PanelLeftClose className="size-4" />}
        </button>
      </div>
    </nav>
  );
}

/** One page link: icon and name, or the icon with the name in a tooltip while collapsed. */
function NavLink({
  to,
  label,
  icon: Icon,
  collapsed,
}: {
  to: string;
  label: string;
  icon: LucideIcon;
  collapsed: boolean;
}) {
  const link = (
    <Link
      to={to}
      className="flex h-8 items-center gap-2.5 rounded-control px-2.5 text-body text-muted transition-colors hover:bg-raised hover:text-ink data-[status=active]:bg-raised data-[status=active]:text-ink [&.active_svg]:text-accent"
      activeProps={{ className: "active" }}
    >
      <Icon className="size-4 shrink-0" />
      {!collapsed && <span className="truncate">{label}</span>}
    </Link>
  );
  return collapsed ? <Tooltip hint={label}>{link}</Tooltip> : link;
}
