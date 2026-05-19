import { plainObject, invertObject } from "@left-curve/utils";
import { getPublicClient } from "./getPublicClient.js";

import type {
  Address,
  AppConfig,
  Flatten,
  Hex,
  PerpsParam,
  PerpsPairParam,
} from "@left-curve/types";
import type { Config } from "../types/store.js";

export type GetAppConfigData = {
  addresses: Flatten<AppConfig["addresses"]> & Record<Address, string>;
  accountFactory: { codeHash: Hex };
  perpsPairs: Record<string, PerpsPairParam>;
  perpsParam: PerpsParam;
} & Omit<AppConfig, "addresses">;

export type GetAppConfigReturnType = Promise<GetAppConfigData>;

export type GetAppConfigErrorType = Error;

export async function getAppConfig<config extends Config>(config: config): GetAppConfigReturnType {
  const client = getPublicClient(config);
  const [appConfig, codeHash, perpsPairs, perpsParam] = await Promise.all([
    client.getAppConfig(),
    client.getCodeHash(),
    client.getPerpsPairParams(),
    client.getPerpsParam(),
  ]);

  const addresses = plainObject(appConfig.addresses) as Flatten<AppConfig["addresses"]>;

  return {
    ...appConfig,
    addresses: {
      ...addresses,
      ...invertObject(addresses),
    },
    accountFactory: { codeHash },
    perpsPairs,
    perpsParam,
  };
}
