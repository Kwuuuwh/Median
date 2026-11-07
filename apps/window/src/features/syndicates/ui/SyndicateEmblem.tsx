import { cn } from "@/shared/lib/cn";
import { emblemOf } from "../model/emblem";

/** A syndicate's emblem; renders nothing when the window has no emblem for it. */
export function SyndicateEmblem({ tag, className }: { tag: string; className?: string }) {
  const emblem = emblemOf(tag);
  if (!emblem) return null;
  return <img src={emblem} alt="" className={cn("shrink-0 object-contain", className)} />;
}
