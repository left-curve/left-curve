import type { Config } from "@left-curve/types";
import { assertDeepEqual } from "@left-curve/utils";
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
