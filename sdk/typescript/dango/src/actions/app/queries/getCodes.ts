import type { Client, CodesResponse } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type GetCodesParameters =
  | {
      startAfter?: string;
      limit?: number;
    }
  | undefined;

export type GetCodesReturnType = Promise<CodesResponse>;

/**
 * Get the codes.
 * @param parameters
 * @param parameters.startAfter The code to start after.
 * @param parameters.limit The number of codes to return.
 * @returns The codes.
 */
export async function getCodes(client: Client, parameters: GetCodesParameters): GetCodesReturnType {
  const { startAfter, limit } = parameters || {};
  const query = {
    codes: { startAfter, limit },
  };

  const res = await queryApp(client, { query });

  if ("codes" in res) return res.codes;
  throw new Error(`expecting codes response, got ${JSON.stringify(res)}`);
}
