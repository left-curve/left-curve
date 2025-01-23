import type {
  Address,
  Coins,
  Transport,
  TxMessageType,
  TypedDataParameter,
} from "@left-curve/types";
import { getCoinsTypedData } from "../../../utils/typedData.js";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type { DangoClient, Signer } from "../../../types/index.js";

export type TransferParameters = {
  sender: Address;
  to: Address;
  coins: Coins;
};

export type TransferReturnType = SignAndBroadcastTxReturnType;

export async function transfer<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: TransferParameters,
): TransferReturnType {
  const { sender, to, coins } = parameters;
  const transferMsg = { transfer: { to, coins } };

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "transfer", type: "Transfer" }],
    extraTypes: {
      Transfer: [
        { name: "to", type: "address" },
        { name: "coins", type: "Coins" },
      ],
      Coins: [...getCoinsTypedData(coins)],
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [transferMsg], typedData });
}
