import type { Client, Transport } from "@left-curve/sdk/types";

import {
  type GetPricesParameters,
  type GetPricesReturnType,
  getPrices,
} from "./queries/getPrices.js";

export type OracleQueryActions = {
  getPrices: (args?: GetPricesParameters) => GetPricesReturnType;
};

export function oracleQueryActions<transport extends Transport = Transport>(
  client: Client<transport>,
): OracleQueryActions {
  return {
    getPrices: (args) => getPrices(client, args),
  };
}
