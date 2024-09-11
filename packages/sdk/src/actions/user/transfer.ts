import type {
  Address,
  Chain,
  Client,
  Coins,
  Hex,
  MessageTypedDataType,
  Signer,
  Transport,
  TypedDataParameter,
} from "@leftcurve/types";
import { getCoinsTypedData } from "@leftcurve/utils";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type TransferParameters = {
  sender: Address;
  to: Address;
  coins: Coins;
};

export type TransferReturnType = Promise<Hex>;

export async function transfer<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: TransferParameters,
): TransferReturnType {
  const { sender, to, coins } = parameters;
  const transferMsg = { transfer: { to, coins } };

  const typedData: TypedDataParameter<MessageTypedDataType> = {
    type: [{ name: "transfer", type: "Transfer" }],
    extraTypes: {
      Transfer: [
        { name: "from", type: "address" },
        { name: "to", type: "address" },
        ...getCoinsTypedData(coins),
      ],
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [transferMsg], typedData });
}
