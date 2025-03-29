import { encodeHex } from "@left-curve/sdk/encoding";

import type { Chain, Client, QueryAbciResponse, Signer, Transport } from "../types/index.js";

export type QueryAbciParameters = {
  path: string;
  data: Uint8Array;
  /** Temporally setted to false because is not possible to prove yet */
  prove?: false;
  height?: number;
};

export type QueryAbciReturnType = Promise<QueryAbciResponse>;

/**
 * Query abci
 * @param parameters
 * @param parameters.path The path to query.
 * @param parameters.data The data to query.
 * @param parameters.prove Whether to prove the query.
 * @param parameters.height The height at which to query the application state.
 * @returns The query response.
 */
export async function queryAbci<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(client: Client<Transport, chain, signer>, parameters: QueryAbciParameters): QueryAbciReturnType {
  const { path, data, height = 0 } = parameters;

  const { response } = await client.request({
    method: "abci_query",
    params: {
      path,
      height: height.toString(),
      data: encodeHex(data),
      prove: false,
    },
  });

  if (response.code === 0) {
    return response;
  }

  throw new Error(
    `query failed! codespace: ${response.codespace}, code: ${response.code}, log: ${response.log}`,
  );
}
