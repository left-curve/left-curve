import { useRef } from "react";

export function useDebounceFn<T extends (...args: any[]) => any>(fn: T, delay: number): T {
  const timeout = useRef<NodeJS.Timeout | null>(null);

  return ((...args: Parameters<T>): void => {
    if (timeout.current) {
      clearTimeout(timeout.current);
    }
    timeout.current = setTimeout(() => {
      fn(...args);
    }, delay);
  }) as T;
}
