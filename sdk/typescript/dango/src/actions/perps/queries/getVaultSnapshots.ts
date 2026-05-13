import { queryWasmSmart } from "../../../index.js";
import type { Client, Prettify } from "../../../types/index.js";

import { getAction, getAppConfig } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { PerpsQueryMsg, VaultSnapshot } from "../../../types/perps.js";

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

  const action = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await action<AppConfig>({});

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
