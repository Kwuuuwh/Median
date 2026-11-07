import { convertFileSrc } from "@tauri-apps/api/core";

/**
 * URL of a game data picture, tagged `?v=<catalog>` so a newer catalog does not get stale cached
 * images; untagged when the catalog version is null.
 */
export function picture(image: string, catalog: string | null): string {
  const address = convertFileSrc(image, "median-img");
  return catalog === null ? address : `${address}?v=${catalog}`;
}
