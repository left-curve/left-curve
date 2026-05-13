import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";

import type { Address, Client, Signer, TypedDataParameter } from "@left-curve/types";
import type { SignAndBroadcastTxReturnType } from "#actions/app/mutations/signAndBroadcastTx.js";

export type WithdrawMarginParameters = {
  sender: Address;
  amount: string;
};

export type WithdrawMarginReturnType = SignAndBroadcastTxReturnType;

export async function withdrawMargin(
  client: Client<Signer>,
  parameters: WithdrawMarginParameters,
): WithdrawMarginReturnType {
  const { sender, amount } = parameters;

  const { addresses } = await getAppConfig(client);

  const msg = {
    trade: {
      withdraw: {
        amount,
      },
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes: {
      Trade: [{ name: "withdraw", type: "Withdraw" }],
      Withdraw: [{ name: "amount", type: "string" }],
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
