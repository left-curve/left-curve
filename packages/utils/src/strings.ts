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
