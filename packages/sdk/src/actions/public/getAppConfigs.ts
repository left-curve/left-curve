import type { Account, Chain, Client, Transport } from "@leftcurve/types";
import type { AppConfigsResponse } from "@leftcurve/types/build/queries";
import { queryApp } from "./queryApp";

export type GetAppConfigsParameters =
  | {
      startAfter?: string;
      limit?: number;
      height?: number;
    }
  | undefined;

export type GetAppConfigsReturnType = Promise<AppConfigsResponse>;

/**
 * Get the app configs.
 * @param parameters
 * @param parameters.startAfter The app config to start after.
 * @param parameters.limit The number of app configs to return.
 * @param parameters.height The height at which to query the app configs.
 * @returns The app configs.
 */
export async function getAppConfigs<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetAppConfigsParameters,
): GetAppConfigsReturnType {
  const { startAfter, limit, height = 0 } = parameters || {};
  const query = {
    appConfigs: { startAfter, limit },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if ("appConfigs" in res) return res.appConfigs;

  throw new Error(`expecting appConfigs response, got ${JSON.stringify(res)}`);
}
