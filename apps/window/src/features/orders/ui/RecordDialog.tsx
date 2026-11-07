import type { OwnOrder } from "@median/ipc";
import { type FormEvent, useState } from "react";
import { useRecordExchange } from "@/shared/data/trading";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";

/** Dialog that records an exchange the game did not report: quantity and price per copy. */
export function RecordDialog({ order, onClose }: { order: OwnOrder | null; onClose: () => void }) {
  const t = useT();
  const record = useRecordExchange();
  const [quantity, setQuantity] = useState("1");
  const [platinum, setPlatinum] = useState("");
  const count = Number.parseInt(quantity, 10);
  const each = Number.parseInt(platinum || String(order?.platinum ?? ""), 10);
  const ready = Number.isFinite(count) && count >= 1 && Number.isFinite(each) && each >= 1;

  function submit(event: FormEvent): void {
    event.preventDefault();
    if (!order || !ready) return;
    record.mutate({ order, quantity: count, platinum: each }, { onSuccess: onClose });
  }

  return (
    <Dialog
      open={order !== null}
      onOpenChange={(open) => {
        if (open) return;
        setQuantity("1");
        setPlatinum("");
        onClose();
      }}
    >
      <DialogContent className="w-80">
        <DialogTitle>{t(order?.selling ? "deal-record-sell" : "deal-record-buy")}</DialogTitle>
        <DialogDescription className="mt-1">{order?.name}</DialogDescription>
        <form className="mt-4 grid gap-3" onSubmit={submit}>
          <label className="grid gap-1 text-muted text-small">
            {t("deal-record-quantity")}
            <Input
              type="number"
              min={1}
              value={quantity}
              onChange={(event) => setQuantity(event.target.value)}
            />
          </label>
          <label className="grid gap-1 text-muted text-small">
            {t("deal-record-price")}
            <Input
              type="number"
              min={1}
              placeholder={String(order?.platinum ?? "")}
              value={platinum}
              onChange={(event) => setPlatinum(event.target.value)}
            />
          </label>
          <Button tone="primary" type="submit" disabled={!ready || record.isPending}>
            {t("deal-record-save")}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
