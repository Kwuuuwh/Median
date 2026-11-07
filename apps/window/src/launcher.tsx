import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { followEvents } from "@/app/events";
import { Launcher } from "@/app/launcher/Launcher";
import { queryClient } from "@/app/query";
import "./styles.css";

async function start(): Promise<void> {
  if (import.meta.env.MODE === "mock") await import("@/mock");

  const root = document.getElementById("root");
  if (!root) throw new Error("launcher.html has no #root");

  await followEvents(queryClient);
  createRoot(root).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <Launcher />
      </QueryClientProvider>
    </StrictMode>,
  );
}

void start();
