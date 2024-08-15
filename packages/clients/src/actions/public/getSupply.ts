import type { Account, Chain, Client, Transport } from "@leftcurve/types";
import { queryApp } from "./queryApp";

export type GetSupplyParameters = {
  denom: string;
  height?: number;
};

export type GetSupplyReturnType = Promise<number>;

export async function getSupply<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(client: Client<Transport, chain, account>, parameters: GetSupplyParameters): GetSupplyReturnType {
  const { denom, height = 0 } = parameters;
  const query = {
    supply: { denom },
  };
  const res = await queryApp<chain, account>(client, { query, height });

  if ("supply" in res) return Number.parseInt(res.supply.amount);
  throw new Error(`expecting supply response, got ${JSON.stringify(res)}`);
}
