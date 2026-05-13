import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Transport } from "@left-curve/sdk/types";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type { AppConfig, DangoClient, Signer, TypedDataParameter } from "../../../types/index.js";

export type DepositMarginParameters = {
  sender: Address;
  amount: string;
};

export type DepositMarginReturnType = SignAndBroadcastTxReturnType;

export async function depositMargin<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: DepositMarginParameters,
): DepositMarginReturnType {
  const { sender, amount } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg = {
    trade: {
      deposit: {},
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "trade", type: "Trade" }],
    extraTypes: {
      Trade: [{ name: "deposit", type: "Deposit" }],
      Deposit: [],
    },
  };

  return await execute(client, {
    sender,
    execute: {
      msg,
      typedData,
      contract: addresses.perps,
      funds: {
        "bridge/usdc": amount,
      },
    },
  });
}
