import { getAppConfig } from "../../../index.js";
import { getAction } from "../../index.js";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Coins, Denom } from "../../../types/index.js";
import type { BroadcastTxSyncReturnType } from "../../app/mutations/broadcastTxSync.js";
import type {
  AppConfig,
  Client,
  DexExecuteMsg,
  Signer,
  TypedDataParameter,
} from "../../../types/index.js";

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

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

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
