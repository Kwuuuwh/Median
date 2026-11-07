import { Dialog, DialogContent } from "@/shared/ui/dialog";
import { openItem, useOpenItem } from "../../model/open";
import { ItemSheet } from "./ItemSheet";

/** Dialog with the card of the item opened by `openItem`; closing it clears the open item. */
export function ItemDialog() {
  const opened = useOpenItem();
  return (
    <Dialog open={opened !== null} onOpenChange={(open) => !open && openItem(null)}>
      <DialogContent place="sheet" className="w-[min(64rem,calc(100vw-3rem))] overflow-hidden">
        {opened && (
          <ItemSheet
            key={`${opened.item}:${opened.variant ?? ""}`}
            item={opened.item}
            variant={opened.variant}
            onOpen={(item) => openItem(item)}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
