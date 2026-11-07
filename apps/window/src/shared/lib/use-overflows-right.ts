import { useLayoutEffect, useState } from "react";

/** Whether the element's content extends past its right edge; updated on scroll and resize. */
export function useOverflowsRight(element: HTMLElement | null): boolean {
  const [overflows, setOverflows] = useState(false);
  useLayoutEffect(() => {
    if (!element) return;
    const measure = () =>
      setOverflows(element.scrollLeft + element.clientWidth < element.scrollWidth - 1);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    element.addEventListener("scroll", measure, { passive: true });
    return () => {
      observer.disconnect();
      element.removeEventListener("scroll", measure);
    };
  }, [element]);
  return overflows;
}
