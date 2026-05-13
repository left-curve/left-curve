import { getAppConfig } from "../../../index.js";
import { getAction } from "../../index.js";
import { execute } from "../../app/mutations/execute.js";

import type { Address } from "../../../types/index.js";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type { AppConfig, Client, Signer, TypedDataParameter } from "../../../types/index.js";

export type VaultRemoveLiquidityParameters = {
  sender: Address;
  sharesToBurn: string;
};

export type VaultRemoveLiquidityReturnType = SignAndBroadcastTxReturnType;

export async function vaultRemoveLiquidity(
  client: Client<Signer>,
  parameters: VaultRemoveLiquidityParameters,
): VaultRemoveLiquidityReturnType {
  const { sender, sharesToBurn } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg = {
    vault: {
      removeLiquidity: {
        sharesToBurn,
      },
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "vault", type: "Vault" }],
    extraTypes: {
      Vault: [{ name: "remove_liquidity", type: "RemoveLiquidity" }],
      RemoveLiquidity: [{ name: "shares_to_burn", type: "uint128" }],
    },
  };

  return await execute(client, {
    sender,
    execute: {
      msg,
      typedData,
      contract: addresses.perps,
    },
  });
}
