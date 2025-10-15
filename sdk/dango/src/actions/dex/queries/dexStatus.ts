import { queryWasmSmart } from "@left-curve/sdk";
import type { Client, Transport } from "@left-curve/sdk/types";

import { getAction, getAppConfig } from "@left-curve/sdk/actions";
import type { Chain, Signer } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/app.js";
import type { DexQueryMsg } from "../../../types/dex.js";

export type DexStatusParameters = {
  height?: number;
};

export type DexStatusReturnType = Promise<boolean>;

/**
 * Queries the DEX status, whether it is paused.
 * @param parameters
 * @param parameters.height The height at which to query the dex status.
 * @returns The DEX status.
 */
export async function dexStatus<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: DexStatusParameters = {},
): DexStatusReturnType {
  const { height = 0 } = parameters;

  const action = getAction(client, getAppConfig, "getAppConfig");

  const msg: DexQueryMsg = {
    paused: {},
  };

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.dex, msg, height });
}
