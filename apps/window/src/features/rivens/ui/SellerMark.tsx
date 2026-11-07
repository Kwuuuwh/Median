import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Tooltip } from "@/shared/ui/tooltip";

const STATUS: Record<string, string> = {
  ingame: "bg-live",
  online: "bg-warn",
  offline: "bg-dim",
};

/** Seller of an auction, with a status dot: in game, online or offline. */
export function SellerMark({ seller, status }: { seller: string; status: string | null }) {
  const t = useT();
  return (
    <Tooltip hint={t("riven-seller-status", { status: status ?? "offline" })}>
      <span className="inline-flex min-w-0 items-center gap-1 text-caption text-muted">
        <span className={cn("size-1.5 shrink-0 rounded-full", STATUS[status ?? ""] ?? "bg-dim")} />
        <span className="truncate">{seller}</span>
      </span>
    </Tooltip>
  );
}
