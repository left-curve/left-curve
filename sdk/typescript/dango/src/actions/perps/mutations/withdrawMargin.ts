import { getAppConfig } from "../../../index.js";
import { getAction } from "../../index.js";
import { execute } from "../../app/mutations/execute.js";

import type { Address } from "../../../types/index.js";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type { AppConfig, Client, Signer, TypedDataParameter } from "../../../types/index.js";

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

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

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
