import { isHex } from "@leftcurve/encoding";
import type { Address } from "@leftcurve/types";

export function isValidAddress(address: Address): boolean {
  if (!address.startsWith("0x")) return false;
  if (address.length !== 42) return false;
  return isHex(address.substring(2));
}
