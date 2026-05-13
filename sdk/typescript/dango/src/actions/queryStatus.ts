import type { ChainStatusResponse, Client } from "../types/index.js";

export type QueryStatusReturnType<T extends ChainStatusResponse = ChainStatusResponse> = Promise<T>;

export async function queryStatus<statusInfo extends ChainStatusResponse>(
  client: Client,
): QueryStatusReturnType<statusInfo> {
  return await client.request({ request: "status" });
}
