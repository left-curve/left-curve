import type { Address } from "@left-curve/sdk/types";
import type { AccountInfo, UserIndexAndName } from "../types/account.js";

export type ToAccountParameters = {
  userIndexAndName: UserIndexAndName;
  address: Address;
  info: AccountInfo;
};

export function toAccount(parameters: ToAccountParameters) {
  const { userIndexAndName, address, info } = parameters;
  const { index, owner } = info;

  return {
    index,
    owner,
    address: address as Address,
    username:
      "name" in userIndexAndName
        ? (userIndexAndName.name as string)
        : `User #${userIndexAndName.index}`,
  };
}
