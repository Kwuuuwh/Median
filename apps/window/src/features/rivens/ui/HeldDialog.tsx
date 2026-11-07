import type { Good, RivenAppraisal } from "@median/ipc";
import type { HeldRiven } from "../model/held";
import { RivenDialog } from "./RivenDialog";
import { RivenSale } from "./RivenSale";

/** Dialog of an inventory riven with its sale form. */
export function HeldDialog({
  held,
  weapon,
  name,
  appraisal,
  onClose,
}: {
  held: HeldRiven | null;
  weapon: Good | undefined;
  name: string | null | undefined;
  appraisal: RivenAppraisal | null | undefined;
  onClose: () => void;
}) {
  return (
    <RivenDialog
      opened={held}
      weapon={weapon}
      name={name}
      appraisal={appraisal}
      trade={(choices) =>
        held && <RivenSale riven={held.riven} rank={held.rank} band={choices?.sell} />
      }
      onClose={onClose}
    />
  );
}
