const size = 256;
let index = size;
let buffer: string;
/**
 * Generates a unique identifier of the specified length.
 *
 * @param length The length of the generated identifier. Default is 11.
 * @returns The generated unique identifier.
 *
 * @remarks
 * This function was forked from [viem](https://github.com/wevm/viem/blob/main/src/utils/uid.ts).
 */
export function uid(length = 11) {
  if (!buffer || index + length > size * 2) {
    buffer = "";
    index = 0;
    for (let i = 0; i < size; i++) {
      buffer += ((256 + Math.random() * 256) | 0).toString(16).substring(1);
    }
  }
  return buffer.substring(index, index++ + length);
}
