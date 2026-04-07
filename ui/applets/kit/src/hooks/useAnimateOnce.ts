import { useEffect, useRef } from "react";

/**
 * Controls animation to only run on initial mount, preventing re-animation on re-renders.
 * Useful for staggered animations that should only play once when content first appears.
 *
 * @param isVisible - Whether the animated content is currently visible
 * @returns Whether the initial animation has already played
 */
export function useAnimateOnce(isVisible: boolean): boolean {
  const hasAnimatedRef = useRef(false);

  useEffect(() => {
    if (isVisible) {
      hasAnimatedRef.current = true;
    }
    return () => {
      hasAnimatedRef.current = false;
    };
  }, [isVisible]);

  return hasAnimatedRef.current;
}
