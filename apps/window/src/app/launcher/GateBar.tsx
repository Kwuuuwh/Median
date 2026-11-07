/** Thin progress bar of a launcher step; `share` runs from 0 to 1. */
export function GateBar({ share }: { share: number }) {
  return (
    <div className="h-1 w-64 overflow-hidden rounded-full bg-raised">
      <div className="h-full bg-accent transition-[width]" style={{ width: `${share * 100}%` }} />
    </div>
  );
}
