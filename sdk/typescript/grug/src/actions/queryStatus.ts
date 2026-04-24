import type {
  Chain,
  ChainStatusResponse,
  Client,
  JsonValue,
  Signer,
  Transport,
} from "../types/index.js";

export type QueryStatusReturnType<T extends ChainStatusResponse> = Promise<T>;

/**
 * Get the chain information.
 * @param parameters
 * @returns The chain information.
 */
export async function queryStatus<
  statusInfo extends JsonValue,
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(client: Client<Transport, chain, signer>): QueryStatusReturnType<statusInfo> {
  return await client.request({ method: "status" });
}
