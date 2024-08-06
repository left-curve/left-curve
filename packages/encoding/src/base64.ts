export function utf8ToBase64(base64: string): string {
	return btoa(base64);
}

export function base64ToUtf8(base64: string): string {
	return atob(base64);
}

export function base64ToBase64Url(base64: string): string {
	return base64.replaceAll("+", "-").replaceAll("/", "_");
}

export function base64UrlToBase64(base64Url: string): string {
	return base64Url.replaceAll("-", "+").replaceAll("_", "/");
}

export function bytesToBase64Url(bytes: Uint8Array): string {
	const base64 = utf8ToBase64(String.fromCharCode(...bytes));
	return base64ToBase64Url(base64);
}

export function base64UrlToBytes(base64Url: string): Uint8Array {
	const base64 = base64UrlToBase64(base64Url);
	const utf8 = base64ToUtf8(base64);
	return Uint8Array.from(utf8, (c) => c.charCodeAt(0));
}
