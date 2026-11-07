import { ChevronDown } from "lucide-react";
import { Select as Primitive } from "radix-ui";
import { cn } from "@/shared/lib/cn";

/** One choice of a select. */
export type Choice<T extends string> = { value: T; label: string };

/** A dropdown of mutually exclusive choices. */
export function Select<T extends string>({
  value,
  choices,
  onChange,
  label,
  className,
}: {
  value: T;
  choices: readonly Choice<T>[];
  onChange: (value: T) => void;
  label: string;
  className?: string;
}) {
  return (
    <Primitive.Root value={value} onValueChange={(next) => onChange(next as T)}>
      <Primitive.Trigger
        aria-label={label}
        className={cn(
          "inline-flex h-8 items-center justify-between gap-1.5 rounded-control border border-line bg-raised px-2.5 text-body text-soft outline-none hover:border-dim hover:text-ink",
          className,
        )}
      >
        <Primitive.Value />
        <ChevronDown className="size-4 text-dim" />
      </Primitive.Trigger>
      <Primitive.Portal>
        <Primitive.Content
          position="popper"
          sideOffset={6}
          className="z-50 max-h-80 min-w-(--radix-select-trigger-width) rounded-panel border border-line bg-raised p-1 shadow-black/40 shadow-xl"
        >
          <Primitive.Viewport>
            {choices.map((choice) => (
              <Primitive.Item
                key={choice.value}
                value={choice.value}
                className="flex h-8 cursor-default items-center rounded-control px-2 text-body text-soft outline-none data-[highlighted]:bg-surface data-[state=checked]:text-accent"
              >
                <Primitive.ItemText>{choice.label}</Primitive.ItemText>
              </Primitive.Item>
            ))}
          </Primitive.Viewport>
        </Primitive.Content>
      </Primitive.Portal>
    </Primitive.Root>
  );
}
