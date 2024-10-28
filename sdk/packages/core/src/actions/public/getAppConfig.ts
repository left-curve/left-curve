import type {
  AppConfigResponse,
  Chain,
  Client,
  JsonValue,
  Signer,
  Transport,
} from "@leftcurve/types";
import { queryApp } from "./queryApp.js";

export type GetAppConfigParameters = {
  key: string;
  height?: number;
};

export type GetAppConfigReturnType<T = JsonValue> = Promise<AppConfigResponse<T>>;

/**
 * Get the application configuration.
 * @param parameters
 * @param parameters.key The key of the application configuration to get.
 * @param parameters.height The height at which to get the application configuration.
 * @returns The application configuration.
 */
export async function getAppConfig<
  value extends JsonValue = JsonValue,
  chain extends Chain | undefined = Chain | undefined,
  signer extends Signer | undefined = Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: GetAppConfigParameters,
): GetAppConfigReturnType<value> {
  const { key, height = 0 } = parameters || {};
  const query = {
    appConfig: { key },
  };
  const res = await queryApp<chain, signer>(client, { query, height });

  if ("appConfig" in res) return res.appConfig as value;

  throw new Error(`expecting appConfig response, got ${JSON.stringify(res)}`);
}
