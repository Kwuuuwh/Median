import "@/mock";
import "../src/styles.css";
import type { Preview } from "@storybook/react-vite";
import { QueryClientProvider } from "@tanstack/react-query";
import { queryClient } from "@/app/query";
import { TooltipProvider } from "@/shared/ui/tooltip";

const preview: Preview = {
  parameters: {
    viewport: {
      options: {
        smallest: {
          name: "Window 1152×648, smallest",
          styles: { width: "1152px", height: "648px" },
          type: "desktop",
        },
        regular: {
          name: "Window 1440×810",
          styles: { width: "1440px", height: "810px" },
          type: "desktop",
        },
        large: {
          name: "Window 1920×1080",
          styles: { width: "1920px", height: "1080px" },
          type: "desktop",
        },
      },
    },
  },
  initialGlobals: { viewport: { value: "regular", isRotated: false } },
  decorators: [
    (Story) => (
      <QueryClientProvider client={queryClient}>
        <TooltipProvider delayDuration={400}>
          <Story />
        </TooltipProvider>
      </QueryClientProvider>
    ),
  ],
};

export default preview;
