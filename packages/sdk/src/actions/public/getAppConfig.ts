import type { Account, Chain, Client, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetAppConfigParameters = {
  key: string;
  height?: number;
};

export type GetAppConfigReturnType<value extends any | undefined> = Promise<value>;

/**
 * Get the application configuration.
 * @param parameters
 * @param parameters.key The key of the application configuration to get.
 * @param parameters.height The height at which to get the application configuration.
 * @returns The application configuration.
 */
export async function getAppConfig<
  value extends any | undefined = any | undefined,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetAppConfigParameters,
): GetAppConfigReturnType<value> {
  const { key, height = 0 } = parameters || {};
  const query = {
    appConfig: { key },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if ("appConfig" in res) return res.appConfig as unknown as value;

  throw new Error(`expecting appConfig response, got ${JSON.stringify(res)}`);
}
