import type { Polarity } from "@median/ipc";
import { Tooltip } from "@/shared/ui/tooltip";

/** Icon of a riven's polarity as the game draws it; the any-polarity icon when it has none. */
export function PolarityMark({ polarity }: { polarity: Polarity | null }) {
  const source = polarity ? `/polarity/${polarity}.svg` : "/polarity/any.webp";
  return (
    <Tooltip hint={polarity ?? ""}>
      <img src={source} alt={polarity ?? ""} className="size-3.5 shrink-0" />
    </Tooltip>
  );
}
