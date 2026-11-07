import { commands, unwrap } from "@median/ipc";
import { useMutation } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { useT } from "@/shared/i18n";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";

/** warframe.market sign-in form (email and password); the backend stores the resulting token. */
export function SignInDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const signIn = useMutation({
    mutationFn: () => unwrap(commands.signIn(email, password)),
    onSuccess: () => {
      setPassword("");
      onOpenChange(false);
    },
  });

  function submit(event: FormEvent): void {
    event.preventDefault();
    signIn.mutate();
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-80">
        <DialogTitle>warframe.market</DialogTitle>
        <DialogDescription className="sr-only">{t("account-sign-in")}</DialogDescription>
        <form className="mt-4 grid gap-3" onSubmit={submit}>
          <label className="grid gap-1 text-muted text-small">
            {t("account-email")}
            <Input
              type="email"
              autoComplete="username"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              required
            />
          </label>
          <label className="grid gap-1 text-muted text-small">
            {t("account-password")}
            <Input
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              required
            />
          </label>
          <Button tone="primary" type="submit" className="mt-1" disabled={signIn.isPending}>
            {t("account-sign-in")}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
