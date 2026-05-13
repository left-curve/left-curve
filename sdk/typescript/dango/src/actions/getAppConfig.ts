import type { AppConfigResponse, Client } from "../types/index.js";
import { getAction } from "./getAction.js";
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
export async function getAppConfig<config extends AppConfigResponse = AppConfigResponse>(
  client: Client,
  parameters: GetAppConfigParameters = {},
): GetAppConfigReturnType<config> {
  const { height = 0 } = parameters;
  const query = {
    appConfig: {},
  };

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if ("appConfig" in res) return res.appConfig as config;

  throw new Error(`expecting appConfig response, got ${JSON.stringify(res)}`);
}
