import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";

import type { Address, Coins, Denom } from "@left-curve/types";
import type { BroadcastTxSyncReturnType } from "#actions/app/mutations/broadcastTxSync.js";
import type { Client, DexExecuteMsg, Signer, TypedDataParameter } from "@left-curve/types";

export type ProvideLiquidityParameters = {
  sender: Address;
  baseDenom: Denom;
  quoteDenom: Denom;
  funds: Coins;
};

export type ProvideLiquidityReturnType = BroadcastTxSyncReturnType;

export async function provideLiquidity(
  client: Client<Signer>,
  parameters: ProvideLiquidityParameters,
): ProvideLiquidityReturnType {
  const { baseDenom, quoteDenom, funds, sender } = parameters;

  const { addresses } = await getAppConfig(client);

  const msg: DexExecuteMsg = {
    provideLiquidity: {
      baseDenom,
      quoteDenom,
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "provide_liquidity", type: "ProvideLiquidity" }],
    extraTypes: {
      ProvideLiquidity: [
        { name: "base_denom", type: "string" },
        { name: "quote_denom", type: "string" },
      ],
    },
  };

  return await execute(client, {
    sender,
    execute: {
      msg,
      typedData,
      contract: addresses.dex,
      funds,
    },
  });
}
