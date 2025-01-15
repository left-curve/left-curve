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
import { getAccountSeenNonces } from "../public/getAccountSeenNonces.js";
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

  const [nonce] = await getAccountSeenNonces(client, { address: sender });

  const data: Metadata = { username, nonce, chainId };

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

  const tx: Tx = {
    sender,
    credential,
    data,
    msgs: messages,
    gasLimit: gasUsed,
  };

  return await broadcastTxSync(client, { tx });
}
