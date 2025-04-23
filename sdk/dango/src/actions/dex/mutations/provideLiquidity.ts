import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "#actions/app/index.js";

import type { Address, Coins, Denom, Transport } from "@left-curve/sdk/types";
import type { BroadcastTxSyncReturnType } from "#actions/app/mutations/broadcastTxSync.js";
import type {
  AppConfig,
  DangoClient,
  DexExecuteMsg,
  Signer,
  TypedDataParameter,
} from "#types/index.js";

export type ProvideLiquidityParameters = {
  sender: Address;
  baseDenom: Denom;
  quoteDenom: Denom;
  funds: Coins;
};

export type ProvideLiquidityReturnType = BroadcastTxSyncReturnType;

export async function provideLiquidity<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: ProvideLiquidityParameters,
): ProvideLiquidityReturnType {
  const { baseDenom, quoteDenom, funds, sender } = parameters;

  const geAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await geAppConfigAction<AppConfig>({});

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
