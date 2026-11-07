import { createRootRoute, createRoute, createRouter, redirect } from "@tanstack/react-router";
import { AgentPage } from "@/features/agent/ui/AgentPage";
import { CatalogPage } from "@/features/catalog/ui/CatalogPage";
import { InventoryPage } from "@/features/inventory/ui/InventoryPage";
import { MasteryPage } from "@/features/mastery/ui/MasteryPage";
import { OrdersPage } from "@/features/orders/ui/OrdersPage";
import { RelicsPage } from "@/features/relics/ui/RelicsPage";
import { RivensPage } from "@/features/rivens/ui/RivensPage";
import { SettingsPage } from "@/features/settings/ui/SettingsPage";
import { TimersPage } from "@/features/timers/ui/TimersPage";
import { AppShell } from "./shell/AppShell";

const rootRoute = createRootRoute({ component: AppShell });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/timers" });
  },
});

const timersRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/timers",
  component: TimersPage,
});

const inventoryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/inventory",
  component: InventoryPage,
});

const ordersRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/orders",
  component: OrdersPage,
});

const agentRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/agent",
  component: AgentPage,
});

const relicsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/relics",
  component: RelicsPage,
});

const rivensRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/rivens",
  component: RivensPage,
});

const catalogRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/catalog",
  component: CatalogPage,
});

const masteryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/mastery",
  component: MasteryPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
});

/** Every page of the window under the shell. */
export const routeTree = rootRoute.addChildren([
  indexRoute,
  timersRoute,
  inventoryRoute,
  ordersRoute,
  agentRoute,
  relicsRoute,
  rivensRoute,
  catalogRoute,
  masteryRoute,
  settingsRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
