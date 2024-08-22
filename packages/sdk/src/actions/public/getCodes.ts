import { decodeBase64 } from "@leftcurve/encoding";
import type { Account, Chain, Client, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetCodesParameters =
  | {
      startAfter?: string;
      limit?: number;
      height?: number;
    }
  | undefined;

export type GetCodesReturnType = Promise<Uint8Array[]>;

/**
 * Get the codes.
 * @param parameters
 * @param parameters.startAfter The code to start after.
 * @param parameters.limit The number of codes to return.
 * @param parameters.height The height at which to query the codes.
 * @returns The codes.
 */
export async function getCodes<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(client: Client<Transport, chain, account>, parameters: GetCodesParameters): GetCodesReturnType {
  const { startAfter, limit, height = 0 } = parameters || {};
  const query = {
    codes: { startAfter, limit },
  };
  const res = await queryApp<chain, account>(client, { query, height });
  if ("codes" in res) return res.codes.map(decodeBase64);
  throw new Error(`expecting codes response, got ${JSON.stringify(res)}`);
}
