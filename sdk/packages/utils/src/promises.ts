/**
 * Sleep for a given number of milliseconds.
 *
 * @param ms - The number of milliseconds to sleep for.
 * @returns A promise that resolves after the given number of milliseconds.
 */
export async function sleep(ms: number) {
  return await new Promise((resolve) => setTimeout(resolve, ms));
}
