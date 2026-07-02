import type { Client, CodeResponse, Hex } from "@left-curve/types";
import { queryApp } from "./queryApp.js";

export type GetCodeParameters = {
  hash: Hex;
};

export type GetCodeReturnType = Promise<CodeResponse>;

/**
 * Get the code.
 * @param parameters
 * @param parameters.hash The hash of the code.
 * @returns The code.
 */
export async function getCode(client: Client, parameters: GetCodeParameters): GetCodeReturnType {
  const { hash } = parameters;
  const query = {
    code: { hash },
  };

  const res = await queryApp(client, { query });

  if ("code" in res) return res.code;
  throw new Error(`expecting code response, got ${JSON.stringify(res)}`);
}
