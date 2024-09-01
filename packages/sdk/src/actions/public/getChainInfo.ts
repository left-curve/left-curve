import type { Chain, Client, InfoResponse, Signer, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetChainInfoParameters =
  | {
      height?: number;
    }
  | undefined;

export type GetChainInfoReturnType = Promise<InfoResponse>;

/**
 * Get the chain information.
 * @param parameters
 * @param parameters.height The height at which to query the chain information.
 * @returns The chain information.
 */
export async function getChainInfo<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetChainInfoParameters,
): GetChainInfoReturnType {
  const { height = 0 } = parameters || {};
  const query = {
    info: {},
  };
  const res = await queryApp<chain, signer>(client, { query, height });

  if ("info" in res) return res.info;

  throw new Error(`expecting info response, got ${JSON.stringify(res)}`);
}
