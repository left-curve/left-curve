import type {
  Address,
  Chain,
  Client,
  Message,
  MessageTypedDataType,
  Metadata,
  Signer,
  Transport,
  Tx,
  TypedDataParameter,
} from "@leftcurve/types";
import { getAccountSequence } from "../public/getAccountSequence";
import { getChainInfo } from "../public/getChainInfo";
import { simulate } from "../public/simulate";
import { type BroadcastTxSyncReturnType, broadcastTxSync } from "./broadcastTxSync";

export type SignAndBroadcastTxParameters = {
  sender: Address;
  messages: Message[];
  gasLimit?: number;
  typedData?: TypedDataParameter<MessageTypedDataType>;
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
