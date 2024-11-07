import type {
  Address,
  Chain,
  Client,
  Coins,
  Signer,
  Transport,
  TxMessageType,
  TypedDataParameter,
} from "@leftcurve/types";
import { getCoinsTypedData } from "@leftcurve/utils";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

export type TransferParameters = {
  sender: Address;
  to: Address;
  coins: Coins;
};

export type TransferReturnType = SignAndBroadcastTxReturnType;

export async function transfer<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
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
