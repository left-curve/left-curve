import type { Result } from "@left-curve/types";

export function tryCatch<R, E = unknown>(fn: () => R): Result<R, E> {
  try {
    const data = fn();
    return { data };
  } catch (error) {
    return { error: error as E };
  }
}
