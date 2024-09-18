export function formatAddress(address: string, sub = 4): string {
  return address.slice(0, 6).concat("...") + address.substring(address.length - sub);
}
