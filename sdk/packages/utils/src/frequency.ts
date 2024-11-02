/**
 * Debounces a function.
 * The function will only be called after it has not been called for a specified amount of time.
 * @param fn The function to debounce.
 * @param delay The amount of time to wait before calling the debounced function.
 * @returns A function that, when called, will call the debounced function after the specified delay.
 */
export function debounce<F extends (...args: any[]) => void>(
  fn: F,
  delay: number,
): (...args: Parameters<F>) => void {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;

  return (...args: Parameters<F>) => {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }

    timeoutId = setTimeout(() => {
      fn(...args);
    }, delay);
  };
}
