import { commands, type Membership, send } from "@median/ipc";
import { ChevronDown } from "lucide-react";
import { DropdownMenu } from "radix-ui";
import { useState } from "react";
import { useT } from "@/shared/i18n";
import { cn } from "@/shared/lib/cn";
import { Button } from "@/shared/ui/button";
import { SignInDialog } from "./SignInDialog";

const STATUSES = ["online", "ingame", "invisible"] as const;

const DOT: Record<string, string> = {
  online: "bg-live",
  ingame: "bg-accent",
  invisible: "bg-dim",
};

/**
 * warframe.market account control: a sign-in button when signed out or expired, and a menu of
 * online status and sign-out when signed in.
 */
export function MarketStatus({ membership }: { membership: Membership }) {
  const t = useT();
  const [signingIn, setSigningIn] = useState(false);

  if (membership.standing !== "in") {
    const expired = membership.standing === "expired";
    return (
      <>
        <Button
          size="sm"
          tone="ghost"
          className={cn(expired && "text-warn")}
          onClick={() => setSigningIn(true)}
        >
          {expired ? t("account-expired") : t("account-sign-in")}
        </Button>
        <SignInDialog open={signingIn} onOpenChange={setSigningIn} />
      </>
    );
  }

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <Button size="sm" tone="ghost" className="gap-2">
          <span className={cn("size-2 rounded-full", DOT[membership.status] ?? "bg-dim")} />
          <span className="text-dim">{t(`status-${membership.status}` as "status-online")}</span>
          {membership.auto && <span className="text-caption text-dim">{t("status-auto")}</span>}
          <ChevronDown className="text-dim" />
        </Button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={6}
          className="z-50 min-w-52 rounded-panel border border-line bg-raised p-1 shadow-black/40 shadow-xl"
        >
          <DropdownMenu.RadioGroup
            value={membership.status}
            onValueChange={(status) => send(commands.setStatus(status))}
          >
            {STATUSES.map((status) => (
              <DropdownMenu.RadioItem
                key={status}
                value={status}
                className="flex h-8 cursor-default items-center gap-2 rounded-control px-2 text-body text-soft outline-none data-[highlighted]:bg-surface data-[state=checked]:text-ink"
              >
                <span className={cn("size-2 rounded-full", DOT[status])} />
                {t(`status-${status}`)}
              </DropdownMenu.RadioItem>
            ))}
          </DropdownMenu.RadioGroup>
          <DropdownMenu.Separator className="my-1 h-px bg-line" />
          <DropdownMenu.CheckboxItem
            checked={membership.auto}
            onCheckedChange={(on) => send(commands.setStatusAuto(on))}
            className="flex h-8 cursor-default items-center justify-between gap-3 rounded-control px-2 text-body text-soft outline-none data-[highlighted]:bg-surface"
          >
            {t("status-auto")}
            <DropdownMenu.ItemIndicator className="text-accent">✓</DropdownMenu.ItemIndicator>
          </DropdownMenu.CheckboxItem>
          <DropdownMenu.Separator className="my-1 h-px bg-line" />
          <DropdownMenu.Item
            onSelect={() => send(commands.signOut())}
            className="flex h-8 cursor-default items-center rounded-control px-2 text-body text-muted outline-none data-[highlighted]:bg-surface data-[highlighted]:text-ink"
          >
            {t("account-sign-out")}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
