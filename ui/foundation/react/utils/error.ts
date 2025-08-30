/**
 * Safely extracts an error message from a value caught in a catch block.
 *
 * Handles cases where the value is:
 * - An instance of Error (uses error.message)
 * - An object with a 'message' property of type string
 * - A string
 * - Any other value (converts it to a string)
 *
 * @param error The value caught in the catch block (of type unknown).
 * @returns A string representing the error message.
 */
export function ensureErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  try {
    if (typeof error === "object" && error !== null) {
      const potentialJson = JSON.stringify(error);

      return potentialJson;
    }
  } catch (_) {}

  return String(error);
}
