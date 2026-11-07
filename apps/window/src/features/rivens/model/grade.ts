import type { RivenGrade } from "@median/ipc";

const RANK: Record<RivenGrade, number> = { bad: 0, potential: 1, good: 2, perfect: 3 };

/** Sort value of a grade, higher for better; -1 for an unknown grade, so it sorts last. */
export function gradeRank(grade: RivenGrade | null | undefined): number {
  return grade ? RANK[grade] : -1;
}

/** A riven to compare: its grade and the price of similar rivens. */
export type Weighed = { grade: RivenGrade | null | undefined; price: number | null | undefined };

/**
 * Whether `fresh` sells better than `old`: by grade when both are known and differ, else by price;
 * null when neither decides.
 */
export function sellsBetter(fresh: Weighed, old: Weighed): boolean | null {
  const [freshRank, oldRank] = [gradeRank(fresh.grade), gradeRank(old.grade)];
  if (freshRank >= 0 && oldRank >= 0 && freshRank !== oldRank) return freshRank > oldRank;
  if (fresh.price == null || old.price == null || fresh.price === old.price) return null;
  return fresh.price > old.price;
}
