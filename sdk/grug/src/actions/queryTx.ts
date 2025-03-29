import type { Base64, Chain, Client, Signer, Transport, TxResponse } from "../types/index.js";

export type QueryTxParameters = {
  hash: Base64;
};

export type QueryTxReturnType = Promise<TxResponse | null>;

/**
 * Query the application state.
 * @param parameters
 * @param parameters.query The query request.
 * @param parameters.height The height at which to query the application state.
 * @returns The query response.
 */
export async function queryTx<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(client: Client<Transport, chain, signer>, parameters: QueryTxParameters): QueryTxReturnType {
  const { hash } = parameters;

  return await client.request({
    method: "tx",
    params: {
      hash,
    },
  });
}
