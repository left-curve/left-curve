import type { AppConfig, Client } from "@left-curve/types";

import { queryApp } from "./queryApp.js";

export type GetAppConfigReturnType = Promise<AppConfig>;

let config: AppConfig | undefined;

/**
 * Get the application configuration.
 * @returns The application configuration.
 */
export async function getAppConfig(client: Client): GetAppConfigReturnType {
  const query = {
    appConfig: {},
  };

  if (config) return config;

  const res = await queryApp(client, { query });

  if ("appConfig" in res) {
    const { appConfig } = res;
    config = appConfig as AppConfig;
    return config;
  }

  throw new Error(`expecting appConfig response, got ${JSON.stringify(res)}`);
}
