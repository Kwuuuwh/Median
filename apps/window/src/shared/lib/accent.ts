import { ACCENTS } from "@median/ipc";

/** Code of an accent the backend accepts. */
export type Accent = (typeof ACCENTS)[number];

/** Hex colour of each accent. */
const PAINT: Readonly<Record<Accent, string>> = {
  ice: "#5aa9e6",
  indigo: "#7c85f2",
  amethyst: "#a78bfa",
  orchid: "#e879b9",
  steel: "#8f9bb3",
  terracotta: "#e0866a",
};

/** Hex colour of an accent; the backend's default accent for an unknown code. */
export function accentPaint(code: string): string {
  return PAINT[code as Accent] ?? PAINT[ACCENTS[0]];
}
