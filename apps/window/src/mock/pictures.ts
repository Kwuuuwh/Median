/** Maps game data picture URLs to the snapshot's `pictures` folder served by the dev server. */
export function servePictures(): void {
  const internals = (window as unknown as { __TAURI_INTERNALS__: Record<string, unknown> })
    .__TAURI_INTERNALS__;
  const fallback = internals.convertFileSrc as (path: string, protocol?: string) => string;
  internals.convertFileSrc = (path: string, protocol = "asset") =>
    protocol === "median-img" ? `/pictures/${path}.webp` : fallback(path, protocol);
}
