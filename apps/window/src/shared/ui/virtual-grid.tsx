import { useVirtualizer } from "@tanstack/react-virtual";
import { Fragment, type ReactNode, useState } from "react";
import { useWidth } from "@/shared/lib/use-width";

/** Space between cells, both ways, in pixels. */
const GAP = 10;

/**
 * Virtualised grid with as many columns as fit; renders only the rows near the visible part of the
 * scroll container.
 */
export function VirtualGrid<T>({
  items,
  scroller,
  least,
  estimate,
  keyOf,
  children,
}: {
  items: readonly T[];
  /** The scroll container; it must be positioned so the grid's offset in it can be measured. */
  scroller: HTMLElement | null;
  /** Minimum cell width in pixels. */
  least: number;
  /** Estimated row height in pixels, used until a row is measured. */
  estimate: number;
  keyOf: (item: T) => string;
  children: (item: T) => ReactNode;
}) {
  "use no memo";
  const [box, setBox] = useState<HTMLDivElement | null>(null);
  const width = useWidth(box);
  const columns = Math.max(1, Math.floor((width + GAP) / (least + GAP)));
  const margin = box?.offsetTop ?? 0;
  const rows = useVirtualizer({
    count: Math.ceil(items.length / columns),
    getScrollElement: () => scroller,
    estimateSize: () => estimate,
    overscan: 3,
    gap: GAP,
    scrollMargin: margin,
  });
  return (
    <div ref={setBox} className="relative" style={{ height: rows.getTotalSize() }}>
      {rows.getVirtualItems().map((row) => (
        <div
          key={`${row.index}:${columns}`}
          data-index={row.index}
          ref={rows.measureElement}
          className="absolute inset-x-0 top-0 grid"
          style={{
            gap: GAP,
            gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
            transform: `translateY(${row.start - margin}px)`,
          }}
        >
          {items.slice(row.index * columns, (row.index + 1) * columns).map((item) => (
            <Fragment key={keyOf(item)}>{children(item)}</Fragment>
          ))}
        </div>
      ))}
    </div>
  );
}
