import { useEffect, useRef } from "react";

import type { DependencyList, RefObject } from "react";

/**
 * Preserves scroll position across re-renders caused by data changes.
 * Useful when content updates (e.g., async data loading) would otherwise reset scroll.
 *
 * @param deps - Dependencies that trigger content changes
 * @returns A ref to attach to the scrollable container
 */
export function usePreserveScroll<T extends HTMLElement = HTMLDivElement>(
  deps: DependencyList = [],
): RefObject<T | null> {
  const scrollRef = useRef<T>(null);
  const scrollPositionRef = useRef(0);

  useEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return;

    // Restore scroll position after render
    if (scrollPositionRef.current > 0) {
      scrollEl.scrollTop = scrollPositionRef.current;
    }

    const handleScroll = () => {
      scrollPositionRef.current = scrollEl.scrollTop;
    };

    scrollEl.addEventListener("scroll", handleScroll);
    return () => scrollEl.removeEventListener("scroll", handleScroll);
  }, deps);

  return scrollRef;
}
