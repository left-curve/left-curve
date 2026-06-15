/**
 * Encode a byte array to a string using the Base64 scheme.
 *
 * JavaScript provides the built-in `btoa` function, but it only works with
 * strings, so we first need to convert the byte array to a Unicode string.
 */
export function encodeBase64(bytes: Uint8Array): string {
  const bitString = String.fromCharCode(...bytes);
  return btoa(bitString);
}

/**
 * Decode a string encoded with the Base64 scheme to a byte array.
 *
 * JavaScript provides the built-in `atob` function, but it only works with
 * strings, so we first need to convert the Unicode string to a byte array.
 */

export function decodeBase64(base64: string) {
  if (!base64.match(/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/)) {
    throw new Error("Invalid base64 string format");
  }
  return Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
}

export function base64UrlToBase64(base64Url: string): string {
  return base64Url.replaceAll("-", "+").replaceAll("_", "/");
}

export function base64ToBase64Url(base64: string): string {
  return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function decodeBase64Url(base64Url: string): Uint8Array {
  const base64 = base64UrlToBase64(base64Url);
  return decodeBase64(base64);
}

export function encodeBase64Url(bytes: Uint8Array): string {
  const base64 = encodeBase64(bytes);
  return base64ToBase64Url(base64);
}
