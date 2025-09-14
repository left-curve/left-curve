import { useEffect } from "react";

export function useDebounce(fn: () => void, deps: unknown[] = [], delay: number = 500) {
  useEffect(() => {
    const handler = setTimeout(() => {
      fn();
    }, delay);

    return () => {
      clearTimeout(handler);
    };
  }, [...deps, delay]);
}
