import { isHex } from "@left-curve/encoding";

export function isValidAddress(address: string): boolean {
  if (!address.startsWith("0x")) return false;
  if (address.length !== 42) return false;
  return isHex(address.substring(2));
}
