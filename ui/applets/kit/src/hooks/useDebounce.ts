import { useEffect } from "react";

export function useDebounce(fn: () => void, delay: number, deps: unknown[] = []) {
  useEffect(() => {
    const handler = setTimeout(() => {
      fn();
    }, delay);

    return () => {
      clearTimeout(handler);
    };
  }, [...deps, delay]);
}
