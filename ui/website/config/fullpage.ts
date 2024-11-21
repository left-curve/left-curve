export function getFullpageLicenseKey() {
  if (!process.env.NEXT_PUBLIC_FULLPAGE_KEY) return "FALLBACK_KEY";
  return new TextDecoder("utf-8", { fatal: true }).decode(
    Uint8Array.from(atob(process.env.NEXT_PUBLIC_FULLPAGE_KEY), (c) => c.charCodeAt(0)),
  );
}
