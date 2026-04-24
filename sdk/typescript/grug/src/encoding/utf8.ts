/**
 * Takes a string and encodes it to a Uint8Array
 */
export function encodeUtf8(str: string) {
  return new TextEncoder().encode(str);
}

/**
 * Takes UTF-8 data and decodes it to a string.
 *
 * In lossy mode, the [REPLACEMENT CHARACTER](https://en.wikipedia.org/wiki/Specials_(Unicode_block))
 * is used to substitude invalid encodings.
 * By default lossy mode is off and invalid data will lead to exceptions.
 */
export function decodeUtf8(data: AllowSharedBufferSource, lossy = false): string {
  const fatal = !lossy;
  return new TextDecoder("utf-8", { fatal }).decode(data);
}
