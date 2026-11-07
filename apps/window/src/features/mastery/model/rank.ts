/** Ranks up to 30 need 2 500 × rank² mastery; each legendary rank above adds a flat 147 500. */
const PLAIN = 30;

/** Total mastery a rank needs. */
export function threshold(rank: number): number {
  return rank <= PLAIN ? 2_500 * rank * rank : 2_250_000 + 147_500 * (rank - PLAIN);
}

/** Highest rank that `total` mastery reaches. */
export function reached(total: number): number {
  let rank = 0;
  while (threshold(rank + 1) <= total) rank += 1;
  return rank;
}
