import type { Chain, Client, CodesResponse, Signer, Transport } from "../types/index.js";
import { getAction } from "./getAction.js";
import { queryApp } from "./queryApp.js";

export type GetCodesParameters =
  | {
      startAfter?: string;
      limit?: number;
      height?: number;
    }
  | undefined;

export type GetCodesReturnType = Promise<CodesResponse>;

/**
 * Get the codes.
 * @param parameters
 * @param parameters.startAfter The code to start after.
 * @param parameters.limit The number of codes to return.
 * @param parameters.height The height at which to query the codes.
 * @returns The codes.
 */
export async function getCodes<chain extends Chain | undefined, signer extends Signer | undefined>(
  client: Client<Transport, chain, signer>,
  parameters: GetCodesParameters,
): GetCodesReturnType {
  const { startAfter, limit, height = 0 } = parameters || {};
  const query = {
    codes: { startAfter, limit },
  };

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if ("codes" in res) return res.codes;
  throw new Error(`expecting codes response, got ${JSON.stringify(res)}`);
}
