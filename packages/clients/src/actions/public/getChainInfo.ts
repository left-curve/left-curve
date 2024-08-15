import type { Account, Chain, Client, InfoResponse, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetChainInfoParameters = {
  height?: number;
};

export type GetChainInfoReturnType = Promise<InfoResponse>;

export async function getChainInfo<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: GetChainInfoParameters,
): GetChainInfoReturnType {
  const { height = 0 } = parameters;
  const query = {
    info: {},
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if ("info" in res) return res.info;

  throw new Error(`expecting info response, got ${JSON.stringify(res)}`);
}
