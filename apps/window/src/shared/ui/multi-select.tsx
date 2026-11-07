import { Check, ChevronDown } from "lucide-react";
import { DropdownMenu } from "radix-ui";
import { cn } from "@/shared/lib/cn";
import type { Choice } from "./select";

/** Dropdown where any number of choices can be selected; the list stays open while selecting. */
export function MultiSelect<T extends string>({
  values,
  choices,
  onChange,
  label,
  className,
}: {
  values: ReadonlySet<T>;
  choices: readonly Choice<T>[];
  onChange: (values: ReadonlySet<T>) => void;
  /** Trigger text while nothing is selected. */
  label: string;
  className?: string;
}) {
  const picked = choices.filter((choice) => values.has(choice.value));
  const shown =
    picked.length === 0
      ? label
      : picked.length === 1
        ? picked[0].label
        : `${picked[0].label} +${picked.length - 1}`;
  const flip = (value: T) => {
    const next = new Set(values);
    if (next.has(value)) next.delete(value);
    else next.add(value);
    onChange(next);
  };
  return (
    <DropdownMenu.Root modal={false}>
      <DropdownMenu.Trigger
        aria-label={label}
        className={cn(
          "inline-flex h-8 items-center justify-between gap-1.5 rounded-control border bg-raised px-2.5 text-body outline-none hover:border-dim hover:text-ink",
          picked.length > 0 ? "border-accent/60 text-ink" : "border-line text-soft",
          className,
        )}
      >
        <span className="truncate">{shown}</span>
        <ChevronDown className="size-4 shrink-0 text-dim" />
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="start"
          sideOffset={6}
          className="z-50 max-h-80 min-w-(--radix-dropdown-menu-trigger-width) overflow-y-auto rounded-panel border border-line bg-raised p-1 shadow-black/40 shadow-xl"
        >
          {choices.map((choice) => (
            <DropdownMenu.CheckboxItem
              key={choice.value}
              checked={values.has(choice.value)}
              onCheckedChange={() => flip(choice.value)}
              onSelect={(event) => event.preventDefault()}
              className="flex h-8 cursor-default items-center gap-2 rounded-control px-2 text-body text-soft outline-none data-[highlighted]:bg-surface data-[state=checked]:text-ink"
            >
              <span className="flex size-4 items-center justify-center rounded-sm border border-line data-[state=checked]:border-accent">
                <DropdownMenu.ItemIndicator>
                  <Check className="size-3 text-accent" />
                </DropdownMenu.ItemIndicator>
              </span>
              {choice.label}
            </DropdownMenu.CheckboxItem>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
