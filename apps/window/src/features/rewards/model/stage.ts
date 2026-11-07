/** Screen size the overlay layout uses: the size the reward screen was measured at. */
const MEASURED_WIDTH = 1920;
const MEASURED_HEIGHT = 1080;

/**
 * Scale of the game's screen relative to 1920×1080, by the tighter of width and height, because
 * the game scales its UI to fit the narrower side: 2560×1080 gives 1.
 */
export function scaleOf(width: number, height: number): number {
  return Math.min(width / MEASURED_WIDTH, height / MEASURED_HEIGHT);
}
