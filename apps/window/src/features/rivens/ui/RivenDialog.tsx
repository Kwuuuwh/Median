import type { Good, Riven, RivenAppraisal, RivenChoices } from "@median/ipc";
import type { ReactNode } from "react";
import { Dialog, DialogContent, DialogTitle } from "@/shared/ui/dialog";
import { RivenSheet } from "./RivenSheet";

/** Dialog with the sheet of the opened riven. */
export function RivenDialog({
  opened,
  weapon,
  name,
  appraisal,
  trade,
  onClose,
}: {
  opened: { key: string; riven: Riven; rank: number } | null;
  weapon: Good | undefined;
  name: string | null | undefined;
  appraisal: RivenAppraisal | null | undefined;
  trade?: (choices: RivenChoices | null | undefined) => ReactNode;
  onClose: () => void;
}) {
  return (
    <Dialog open={opened !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent place="sheet" className="w-[min(72rem,calc(100vw-3rem))] overflow-y-auto p-5">
        <DialogTitle className="sr-only">{name ?? weapon?.name ?? ""}</DialogTitle>
        {opened && (
          <RivenSheet
            key={opened.key}
            riven={opened.riven}
            rank={opened.rank}
            weapon={weapon}
            name={name}
            appraisal={appraisal}
            trade={trade}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
