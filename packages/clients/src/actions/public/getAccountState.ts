/*
  async getAccountState(address: string, height = 0): Promise<AccountStateResponse> {
    const accountSate = await this.queryWasmSmart<AccountStateResponse>(
      address,
      { state: {} },
      height,
    );

    return accountSate;
  }


*/

import type {
  Account,
  AccountStateResponse,
  Chain,
  Client,
  InfoResponse,
  Transport,
} from "@leftcurve/types";
import { queryWasmSmart } from "./queryWasmSmart";

export type GetAccountStateParameters = {
  address: string;
  height?: number;
};

export type GetAccountStateReturnType = Promise<AccountStateResponse>;

export async function getAccountState<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetAccountStateParameters,
): GetAccountStateReturnType {
  const { address, height = 0 } = parameters;
  const msg = { state: {} };
  return await queryWasmSmart<AccountStateResponse, chain, account>(client, {
    contract: address,
    msg,
    height,
  });
}
