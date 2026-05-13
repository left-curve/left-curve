import type { Client } from "../../../types/index.js";

import { getAction, queryApp } from "../../index.js";
import type { AppConfig } from "../../../types/index.js";

export type GetAppConfigParameters = {
  height?: number;
};

export type GetAppConfigReturnType = Promise<AppConfig>;

let config: AppConfig | undefined;

/**
 * Get the application configuration.
 * @param parameters
 * @param parameters.height The height at which to get the application configuration.
 * @returns The application configuration.
 */
export async function getAppConfig(
  client: Client,
  parameters: GetAppConfigParameters = {},
): GetAppConfigReturnType {
  const { height = 0 } = parameters;
  const query = {
    appConfig: {},
  };

  if (config) return config;

  const action = getAction(client, queryApp, "queryApp");

  const res = await action({ query, height });

  if ("appConfig" in res) {
    const { appConfig } = res;
    config = appConfig as AppConfig;
    return config;
  }

  throw new Error(`expecting appConfig response, got ${JSON.stringify(res)}`);
}
