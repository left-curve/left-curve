import type { Client } from "@left-curve/types";

import {
  type GetPricesParameters,
  type GetPricesReturnType,
  getPrices,
} from "./queries/getPrices.js";

export type OracleQueryActions = {
  getPrices: (args?: GetPricesParameters) => GetPricesReturnType;
};

export function oracleQueryActions(client: Client): OracleQueryActions {
  return {
    getPrices: (args) => getPrices(client, args),
  };
}
