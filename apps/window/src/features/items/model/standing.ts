/** The account's progress with an item: copies held, mastery, and parts held. */
export type Standing = {
  /** Copies held; for an item built from parts, copies of the built item. */
  held: number;
  /** Whether the account has mastered the item, ranking it to its top rank. */
  mastered: boolean;
  /** Parts held out of the total, for an item built from a set. */
  parts: { owned: number; total: number } | null;
};

/** Whether the account needs nothing more of the item: it is held or mastered. */
export function isSettled(standing: Standing): boolean {
  return standing.held > 0 || standing.mastered;
}
