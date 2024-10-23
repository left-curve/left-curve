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

/**
 * Truncate an address.
 * @param address The address to truncate.
 * @param substring The number of characters to show at the end.
 * @returns The truncate address.
 */
export function truncateAddress(address: string, substring = 4): string {
  return address.slice(0, 6).concat("...") + address.substring(address.length - substring);
}
