import { queryWasmSmart } from "#actions/app/queries/queryWasmSmart.js";
import type { Client, PerpsQueryMsg, Prettify, VaultSnapshot } from "@left-curve/types";

import { getAppConfig } from "#actions/app/queries/getAppConfig.js";

export type GetVaultSnapshotsParameters = Prettify<{
  min?: number;
  max?: number;
  height?: number;
}>;

export type GetVaultSnapshotsReturnType = Promise<Record<string, VaultSnapshot>>;

export async function getVaultSnapshots(
  client: Client,
  parameters?: GetVaultSnapshotsParameters,
): GetVaultSnapshotsReturnType {
  const { min, max, height = 0 } = parameters ?? {};

  const { addresses } = await getAppConfig(client);

  const msg: PerpsQueryMsg = {
    vaultSnapshots: {
      min: min != null ? String(min) : undefined,
      max: max != null ? String(max) : undefined,
    },
  };

  return queryWasmSmart(client, {
    contract: addresses.perps,
    msg,
    height,
  });
}
