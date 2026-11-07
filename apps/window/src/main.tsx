import { commands } from "@median/ipc";
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { followEvents } from "@/app/events";
import { queryClient } from "@/app/query";
import { router } from "@/app/router";
import "./styles.css";

async function start(): Promise<void> {
  if (import.meta.env.MODE === "mock") await import("@/mock");

  const root = document.getElementById("root");
  if (!root) throw new Error("index.html has no #root");

  await followEvents(queryClient);
  createRoot(root).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </StrictMode>,
  );
  requestAnimationFrame(() => void commands.windowReady());
}

void start();
