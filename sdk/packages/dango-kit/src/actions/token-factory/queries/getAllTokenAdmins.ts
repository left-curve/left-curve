import { getAppConfig } from "../../public/getAppConfig.js";
import { queryWasmSmart } from "../../public/queryWasmSmart.js";

import type {
  Address,
  Chain,
  Client,
  Denom,
  Signer,
  TokenFactoryQueryMsg,
  Transport,
} from "@left-curve/types";
import type { DangoAppConfigResponse } from "@left-curve/types/dango";

export type GetAllTokenAdminsParameters = {
  startAfter?: Denom;
  limit?: number;
  height?: number;
};

export type GetAllTokenAdminsReturnType = Promise<Record<Denom, Address>>;

/**
 * Enumerate all denoms and their admin addresses.
 * @param parameters
 * @param parameters.startAfter The denom to start after.
 * @param parameters.limit The maximum number of elments to return.
 * @param parameters.height The height to query the TokenFactory's admins.
 * @return a map of denoms to their admin addresses.
 */
export async function getAllTokenAdmins<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAllTokenAdminsParameters,
): GetAllTokenAdminsReturnType {
  const { startAfter, limit, height = 0 } = parameters;
  const msg: TokenFactoryQueryMsg = { admins: { startAfter, limit } };

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await queryWasmSmart(client, { contract: addresses.tokenFactory, msg, height });
}
