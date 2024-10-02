import { decodeBase64 } from "@leftcurve/encoding";
import type { Chain, Client, Hex, Signer, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetCodeParameters = {
  hash: Hex;
  height?: number;
};

export type GetCodeReturnType = Promise<Uint8Array>;

/**
 * Get the code.
 * @param parameters
 * @param parameters.hash The hash of the code.
 * @param parameters.height The height at which to query the code.
 * @returns The code.
 */
export async function getCode<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetCodeParameters,
): GetCodeReturnType {
  const { hash, height = 0 } = parameters;
  const query = {
    code: { hash },
  };
  const res = await queryApp<chain, signer>(client, { query, height });
  if ("code" in res) return decodeBase64(res.code);
  throw new Error(`expecting code response, got ${JSON.stringify(res)}`);
}
