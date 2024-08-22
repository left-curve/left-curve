import type {
  Account,
  AccountId,
  AccountInfo,
  Address,
  Chain,
  Client,
  Transport,
} from "@leftcurve/types";
import { getAppConfig } from "./getAppConfig";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetAccountInfoByAccountIdParameters = {
  accountId: AccountId;
  height?: number;
};

export type GetAccountInfoByAccountIdReturnType = Promise<AccountInfo>;

/**
 * Get the account information by account id.
 * @param parameters
 * @param parameters.accountId The account id to get information for.
 * @param parameters.height The height at which to get the account information.
 * @returns The account information.
 */
export async function getAccountInfoByAccountId<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetAccountInfoByAccountIdParameters,
): GetAccountInfoByAccountIdReturnType {
  const { accountId, height = 0 } = parameters;
  const msg = { account: accountId };

  const accountFactory = await getAppConfig<Address>(client, { key: "account_factory" });

  return await queryWasmSmart(client, { contract: accountFactory, msg, height });
}
