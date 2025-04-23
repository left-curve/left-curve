import { getCoinsTypedData } from "#utils/typedData.js";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type { Address, Coins, Transport } from "@left-curve/sdk/types";

import type { DangoClient, Signer, TxMessageType, TypedDataParameter } from "#types/index.js";

export type TransferParameters = {
  sender: Address;
  transfer: Record<Address, Coins>;
};

export type TransferReturnType = SignAndBroadcastTxReturnType;

export async function transfer<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: TransferParameters,
): TransferReturnType {
  const { sender, transfer } = parameters;
  const transferMsg = { transfer };

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "transfer", type: "Transfer" }],
    extraTypes: Object.entries(transfer).reduce(
      (acc, [address, coins], i) => {
        acc.Transfer.push({ name: address, type: `Coin${i}` });
        return Object.assign(acc, { [`Coin${i}`]: getCoinsTypedData(coins) });
      },
      Object.assign({ Transfer: [] }),
    ),
  };

  return await signAndBroadcastTx(client, { sender, messages: [transferMsg], typedData });
}
