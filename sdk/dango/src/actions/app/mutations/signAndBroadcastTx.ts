import { getChainInfo, simulate } from "@left-curve/sdk";
import type { Address, Message, Transport } from "@left-curve/sdk/types";

import { getAccountSeenNonces } from "../../account-factory/queries/getAccountSeenNonces.js";
import { type BroadcastTxSyncReturnType, broadcastTxSync } from "./broadcastTxSync.js";

import type {
  DangoClient,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "../../../types/index.js";

export type SignAndBroadcastTxParameters = {
  sender: Address;
  messages: Message[];
  gasLimit?: number;
  typedData?: TypedDataParameter<TxMessageType>;
};

export type SignAndBroadcastTxReturnType = BroadcastTxSyncReturnType;

export async function signAndBroadcastTx<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SignAndBroadcastTxParameters,
): SignAndBroadcastTxReturnType {
  if (!client.signer) throw new Error("client must have a signer");
  const { messages, sender, typedData, gasLimit: gas } = parameters;

  const chainId = await (async () => {
    if (client.chain?.id) return client.chain.id;
    const { chainId } = await getChainInfo(client);
    return chainId;
  })();

  const { username } = client;

  if (!username) {
    throw new Error("client must have a username");
  }

  const [nonce] = await getAccountSeenNonces(client, { address: sender });

  const data = { username, nonce, chainId };

  const { gasUsed } = gas
    ? { gasUsed: gas }
    : await simulate(client, { simulate: { sender, msgs: messages, data } });

  const { credential } = await client.signer.signTx(
    {
      sender,
      messages,
      data,
      gasLimit: gasUsed,
    },
    { typedData },
  );

  const tx = {
    sender,
    credential,
    data,
    msgs: messages,
    gasLimit: gasUsed,
  };

  return await broadcastTxSync(client, { tx });
}
