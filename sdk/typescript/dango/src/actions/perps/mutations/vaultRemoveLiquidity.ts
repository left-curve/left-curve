import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";

import type { Address, Client, Signer, TypedDataParameter } from "@left-curve/types";
import type { SignAndBroadcastTxReturnType } from "#actions/app/mutations/signAndBroadcastTx.js";

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

  const { addresses } = await getAppConfig(client);

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
