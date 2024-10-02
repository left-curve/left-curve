/**
 * Convert a string from camelCase to snake_case.
 */
export function camelToSnake(str: string): string {
  return str.replace(/([A-Z])/g, "_$1").toLowerCase();
}

/**
 * Convert a string from snake_case to camelCase.
 */
export function snakeToCamel(str: string): string {
  return str.replace(/(_[a-z])/g, (group) => group.toUpperCase().replace("_", ""));
}

/**
 * Capitalize the first letter of a string.
 * @param str The string to capitalize.
 * @returns The capitalized string.
 */
export function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
