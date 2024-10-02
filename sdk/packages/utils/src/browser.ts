/**
 * Get the browser OS from the user agent
 */
export function getNavigatorOS(): string {
  const { userAgent } = navigator;
  if (userAgent.indexOf("Windows") !== -1) return "Windows";
  if (userAgent.indexOf("Macintosh") !== -1) return "MacOS";
  if (userAgent.indexOf("Linux") !== -1) return "Linux";
  if (userAgent.indexOf("Android") !== -1) return "Android";
  if (/iPad|iPhone|iPod/.test(userAgent)) return "iOS";
  if (userAgent.indexOf("CrOS") !== -1) return "ChromeOS";
  if (userAgent.indexOf("Tizen") !== -1) return "Tizen";
  return "Unknown";
}
