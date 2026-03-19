import { plainObject, invertObject } from "@left-curve/dango/utils";
import { getPublicClient } from "./getPublicClient.js";

import type { Address, AppConfig, Denom, Flatten, Hex, PairUpdate } from "@left-curve/dango/types";
import type { Config } from "../types/store.js";

export type GetAppConfigData = {
  addresses: Flatten<AppConfig["addresses"]> & Record<Address, string>;
  accountFactory: { codeHash: Hex };
  pairs: Record<Denom, PairUpdate>;
} & Omit<AppConfig, "addresses">;

export type GetAppConfigReturnType = Promise<GetAppConfigData>;

export type GetAppConfigErrorType = Error;

export async function getAppConfig<config extends Config>(config: config): GetAppConfigReturnType {
  const client = getPublicClient(config);
  const [appConfig, codeHash, pairs] = await Promise.all([
    client.getAppConfig(),
    client.getCodeHash(),
    client.getPairs(),
  ]);

  const addresses = plainObject(appConfig.addresses) as Flatten<AppConfig["addresses"]>;

  return {
    ...appConfig,
    addresses: {
      ...addresses,
      ...invertObject(addresses),
    },
    accountFactory: { codeHash },
    pairs: pairs.reduce(
      (acc, pair) => {
        acc[pair.baseDenom] = pair;
        return acc;
      },
      Object.create({}) as Record<Denom, PairUpdate>,
    ),
  };
}
