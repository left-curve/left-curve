import { assertDeepEqual } from "@left-curve/dango/utils";

import type { Config } from "../types/store.js";
import { type GetAccountReturnType, getAccount } from "./getAccount.js";

export type WatchAccountParameters = {
  onChange(account: GetAccountReturnType, prevAccount: GetAccountReturnType): void;
};

export type WatchAccountReturnType = () => void;

export function watchAccount(
  config: Config,
  parameters: WatchAccountParameters,
): WatchAccountReturnType {
  const { onChange } = parameters;

  return config.subscribe(() => getAccount(config), onChange, {
    equalityFn(a, b) {
      const { connector: aConnector, ...aRest } = a;
      const { connector: bConnector, ...bRest } = b;
      return (
        assertDeepEqual(aRest, bRest) &&
        aConnector?.id === bConnector?.id &&
        aConnector?.uid === bConnector?.uid
      );
    },
  });
}
