/**
 * Represents either an JSON object, an array, a string, a number, a null, an undefined or a boolean.
 * Note that we utilize a recursive type definition here.
 */
export type Json = { [key: string]: Json } | Json[] | string | number | boolean | undefined | null;

/**
 * Represents a string in hex.
 */
export type Hex = string;

/**
 * Represents a string in base64.
 */
export type Base64 = string;
