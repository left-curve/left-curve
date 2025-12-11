import type { Address } from "@left-curve/sdk/types";
import type { AccountInfo, AccountTypes, UserIndexAndName } from "../types/account.js";

export type ToAccountParameters = {
  userIndexAndName: UserIndexAndName;
  address: Address;
  info: AccountInfo;
};

export function toAccount(parameters: ToAccountParameters) {
  const { userIndexAndName, address, info } = parameters;
  const { index, params } = info;

  const type = Object.keys(params)[0] as AccountTypes;
  return {
    index,
    params,
    address: address as Address,
    username:
      "name" in userIndexAndName
        ? (userIndexAndName.name as string)
        : `User #${userIndexAndName.index}`,
    type: type,
  };
}
