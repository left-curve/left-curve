import type { Address, Chain, Client, Hex, Signer, Transport, Username } from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetKeyIdByKeyHashParameters = {
  hash: Hex;
  height?: number;
};

export type GetKeyIdByKeyHashReturnType = Promise<Username[]>;

/**
 * Given a key hash, get the key id.
 * @param parameters
 * @param parameters.hash The key hash of the account.
 * @returns keyId associated with the key hash.
 */
export async function getKeyIdByKeyHash<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetKeyIdByKeyHashParameters,
): GetKeyIdByKeyHashReturnType {
  const { hash, height = 0 } = parameters;
  const msg = { keyId: { hash } };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
