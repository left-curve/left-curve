import { eip1193 } from "./eip1193.js";

import type { EIP6963ProviderDetail } from "../types/eip6963.js";

export function eip6963({ info, provider }: EIP6963ProviderDetail) {
  const { name, icon } = info;
  return eip1193({
    id: name.toLowerCase(),
    name,
    icon,
    provider: () => provider,
  });
}
