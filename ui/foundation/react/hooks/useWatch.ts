import { useEffect } from "react";

/**
 * A custom hook that triggers a callback function whenever the value changes.
 * @param v The value to watch.
 * @param trigger The callback function to trigger when the value changes.
 */
export function useWatchEffect<T = unknown>(v: T, trigger: (v: T) => void): void {
  useEffect(() => {
    trigger(v);
  }, [v]);
}
