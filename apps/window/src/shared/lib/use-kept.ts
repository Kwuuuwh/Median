import { useState } from "react";

/** Last value of each kept state by key, held in memory until the window closes. */
const kept = new Map<string, unknown>();

/** `useState` that keeps its value under `key` across unmounts, so a reopened page restores it. */
export function useKept<T>(key: string, initial: T): [T, (next: T | ((before: T) => T)) => void] {
  const [value, setValue] = useState<T>(() => (kept.has(key) ? (kept.get(key) as T) : initial));
  const set = (next: T | ((before: T) => T)) => {
    setValue((before) => {
      const value = typeof next === "function" ? (next as (before: T) => T)(before) : next;
      kept.set(key, value);
      return value;
    });
  };
  return [value, set];
}
