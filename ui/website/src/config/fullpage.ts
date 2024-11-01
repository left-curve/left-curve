export function getFullpageLicenseKey() {
  if (!import.meta.env.PUBLIC_FULLPAGE_KEY) return "FALLBACK_KEY";
  return new TextDecoder("utf-8", { fatal: true }).decode(
    Uint8Array.from(atob(import.meta.env.PUBLIC_FULLPAGE_KEY), (c) => c.charCodeAt(0)),
  );
}
