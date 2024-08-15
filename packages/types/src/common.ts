/**
 * Represents either an JSON object, an array, a string, a number, a null, an undefined or a boolean.
 * Note that we utilize a recursive type definition here.
 */
export type Json = { [key: string]: Json } | Json[] | string | number | boolean | undefined | null;

/**
 * Represents a string or an Uint8Array encoded in hex.
 */
export type Hex = Uint8Array | string;

/**
 * Represents a string or an Uint8Array encoded in base64.
 */
export type Base64 = Uint8Array | string;
