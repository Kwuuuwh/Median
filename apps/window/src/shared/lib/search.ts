/** Text normalised for search: lower case, with ё replaced by е. */
function fold(text: string): string {
  return text.toLocaleLowerCase("ru").replaceAll("ё", "е");
}

/** Predicate that is true when a name contains every word of the search, in any order. */
export function matcher(search: string): (name: string) => boolean {
  const words = fold(search).split(/\s+/).filter(Boolean);
  return (name) => words.every((word) => fold(name).includes(word));
}
