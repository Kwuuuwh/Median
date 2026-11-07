import type { Meta, StoryObj } from "@storybook/react-vite";
import { createMemoryHistory, createRouter, RouterProvider } from "@tanstack/react-router";
import { useMemo } from "react";
import { routeTree } from "@/app/router";

/** The whole window on the page at `path`. */
function Window({ path }: { path: string }) {
  const router = useMemo(
    () => createRouter({ routeTree, history: createMemoryHistory({ initialEntries: [path] }) }),
    [path],
  );
  return (
    <div className="h-screen">
      <RouterProvider router={router} />
    </div>
  );
}

const meta = {
  title: "Window",
  component: Window,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof Window>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Timers: Story = { args: { path: "/timers" } };

export const Inventory: Story = { args: { path: "/inventory" } };

export const Orders: Story = { args: { path: "/orders" } };

export const Agent: Story = { args: { path: "/agent" } };

export const Relics: Story = { args: { path: "/relics" } };

export const Catalog: Story = { args: { path: "/catalog" } };

export const Mastery: Story = { args: { path: "/mastery" } };

export const Settings: Story = { args: { path: "/settings" } };
