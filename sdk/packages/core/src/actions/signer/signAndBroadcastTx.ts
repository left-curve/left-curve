import type {
  Address,
  Chain,
  Client,
  Message,
  Metadata,
  Signer,
  Transport,
  Tx,
  TxMessageType,
  TypedDataParameter,
} from "@left-curve/types";
import { getAccountSequence } from "../public/getAccountSequence.js";
import { getChainInfo } from "../public/getChainInfo.js";
import { simulate } from "../public/simulate.js";
import { type BroadcastTxSyncReturnType, broadcastTxSync } from "./broadcastTxSync.js";

export type SignAndBroadcastTxParameters = {
  sender: Address;
  messages: Message[];
  gasLimit?: number;
  typedData?: TypedDataParameter<TxMessageType>;
};

export type SignAndBroadcastTxReturnType = BroadcastTxSyncReturnType;

export async function signAndBroadcastTx<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: SignAndBroadcastTxParameters,
): SignAndBroadcastTxReturnType {
  if (!client.signer) throw new Error("client must have a signer");
  const { messages, sender, typedData, gasLimit: gas } = parameters;

  const chainId = await (async () => {
    if (client.chain?.id) return client.chain.id;
    const { chainId } = await getChainInfo(client, {});
    return chainId;
  })();

  const { username } = client;

  if (!username) {
    throw new Error("client must have a username");
  }

  const sequence = await getAccountSequence(client, { address: sender }).catch(() => 0);

  const { credential, keyHash } = await client.signer.signTx({
    sender,
    chainId,
    messages,
    sequence,
    typedData,
  });

  const data: Metadata = { keyHash, username, sequence };

  const { gasUsed } = gas
    ? { gasUsed: gas }
    : await simulate(client, { simulate: { sender, msgs: messages, data } });

  const tx: Tx = {
    sender,
    credential,
    data,
    msgs: messages,
    gasLimit: gasUsed,
  };

  return await broadcastTxSync(client, { tx });
}
