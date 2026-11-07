import type { CSSProperties } from "react";

/** Screen width in pixels the positions below were measured at; matches reward/cards.rs. */
const MEASURED = 1920;

/** Maximum number of cards on the reward screen. */
const CARDS = 4;

/** Left edge in pixels of the first card of a full squad, and the distance to the next card. */
const FIRST = 478;
const STEP = 243;

/** Centre of the card row, which the game keeps fixed for any number of cards. */
const CENTRE = FIRST + (STEP * CARDS) / 2;

/**
 * Position of a row of `cards` on the overlay, in percent of the screen width. A shorter row moves
 * right by half a step per missing card, so the panel matches the cards instead of a full squad.
 */
export function row(cards: number): CSSProperties {
  const wide = STEP * cards;
  return {
    left: `${((CENTRE - wide / 2) / MEASURED) * 100}%`,
    width: `${(wide / MEASURED) * 100}%`,
    gridTemplateColumns: `repeat(${cards}, 1fr)`,
  };
}
