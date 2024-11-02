import type { EIP6963ProviderDetail } from "@leftcurve/types";
import { eip1193 } from "./eip1193.js";

export function eip6963({ info, provider }: EIP6963ProviderDetail) {
  const { name, icon } = info;
  return eip1193({
    id: name.toLowerCase(),
    name,
    icon,
    provider: () => provider,
  });
}
