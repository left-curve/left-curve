import type { Address } from "@left-curve/sdk/types";
import type { AccountInfo, AccountTypes } from "../types/account.js";

export type ToAccountParameters = {
  username: string;
  address: Address;
  info: AccountInfo;
};

export function toAccount(parameters: ToAccountParameters) {
  const { username, address, info } = parameters;
  const { index, params } = info;

  const type = Object.keys(params)[0] as AccountTypes;
  return {
    index,
    params,
    address: address as Address,
    username,
    type: type,
  };
}
