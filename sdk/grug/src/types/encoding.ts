/**
 * Represents a JSON object.
 */
export type Json = { [key: string]: JsonValue };

/**
 * Represents either an JSON object, an array, a string, a number, a null,
 * an undefined or a boolean.
 */
export type JsonValue = Json | JsonValue[] | string | number | boolean | undefined | null;

/**
 * Represents a JSON string.
 */
export type JsonString = string;

/**
 * Represents a string in hex.
 */
export type Hex = string;

/**
 * Represents a string in base64.
 */
export type Base64 = string;

export type Binary = Uint8Array;

export type Encoder = {
  encode(): Uint8Array;
};
