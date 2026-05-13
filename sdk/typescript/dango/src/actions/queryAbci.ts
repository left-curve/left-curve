import { encodeHex } from "../encoding/hex.js";

import type { Client, QueryAbciResponse } from "../types/index.js";

export type QueryAbciParameters = {
  path: string;
  data: Uint8Array;
  prove?: false;
  height?: number;
};

export type QueryAbciReturnType = Promise<QueryAbciResponse>;

export async function queryAbci(
  client: Client,
  parameters: QueryAbciParameters,
): QueryAbciReturnType {
  const { path, data, height = 0 } = parameters;

  const { response } = await client.request<{ response: QueryAbciResponse }>({
    request: "abci_query",
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
