import type { AppConfigResponse, Chain, Client, Signer, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp.js";

export type GetAppConfigParameters = {
  height?: number;
};

export type GetAppConfigReturnType<T = AppConfigResponse> = Promise<T>;

/**
 * Get the application configuration.
 * @param parameters
 * @param parameters.height The height at which to get the application configuration.
 * @returns The application configuration.
 */
export async function getAppConfig<
  config extends AppConfigResponse = AppConfigResponse,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAppConfigParameters = {},
): GetAppConfigReturnType<config> {
  const { height = 0 } = parameters;
  const query = {
    appConfig: {},
  };
  const res = await queryApp<chain, signer>(client, { query, height });

  if ("appConfig" in res) return res.appConfig as config;

  throw new Error(`expecting appConfig response, got ${JSON.stringify(res)}`);
}
