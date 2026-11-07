import { type ClassValue, clsx } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

/** Tailwind merge with the window's own text sizes and radii, so they are not merged as colours. */
const merge = extendTailwindMerge({
  extend: {
    theme: {
      text: ["caption", "small", "body", "title", "heading", "figure"],
      radius: ["control", "panel"],
    },
  },
});

/** Joined class names; a later conflicting Tailwind utility overrides an earlier one. */
export function cn(...inputs: ClassValue[]): string {
  return merge(clsx(inputs));
}
