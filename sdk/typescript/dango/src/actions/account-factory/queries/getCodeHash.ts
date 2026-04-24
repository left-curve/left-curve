import { getAppConfig, queryWasmSmart } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import type { Chain, Client, Hex, Signer, Transport } from "@left-curve/sdk/types";
import type { AppConfig } from "../../../types/index.js";

export type GetCodeHashParameters = {
  height?: number;
};

export type GetCodeHashReturnType = Promise<Hex>;

/**
 * Get the account code hash.
 * @param parameters
 * @param parameters.height The height at which to query the code hash.
 * @returns The account code hash.
 */
export async function getCodeHash<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters?: GetCodeHashParameters,
): GetCodeHashReturnType {
  const { height = 0 } = parameters || {};
  const msg = { codeHash: {} };
  const action = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await action<AppConfig>({});

  return await queryWasmSmart(client, { contract: addresses.accountFactory, msg, height });
}
