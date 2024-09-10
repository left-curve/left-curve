import type {
  Address,
  Chain,
  Client,
  Hex,
  Message,
  Metadata,
  Signer,
  Transport,
} from "@leftcurve/types";
import { getAccountSequence } from "../public/getAccountSequence";
import { getChainInfo } from "../public/getChainInfo";
import { simulate } from "../public/simulate";

export type SignAndBroadcastTxParameters = {
  sender: Address;
  msgs: Message[];
  gasLimit?: number;
};

export type SignAndBroadcastTxReturnType = Promise<Hex>;

export async function signAndBroadcastTx<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: SignAndBroadcastTxParameters,
): SignAndBroadcastTxReturnType {
  if (!client.signer) throw new Error("client must have a signer");
  const { msgs, sender, gasLimit: gas } = parameters;
  let chainId = client.chain?.id;

  if (!chainId) {
    const response = await getChainInfo(client, {});
    chainId = response.chainId;
  }

  const { username } = client;

  if (!username) {
    throw new Error("client must have a username");
  }

  const sequence = await getAccountSequence(client, { address: sender }).catch(() => 0);

  const { credential, keyHash } = await client.signer.signTx({
    chainId,
    msgs,
    sequence,
  });

  const data: Metadata = { keyHash, username, sequence };

  const { gasLimit } = gas
    ? { gasLimit: gas }
    : await simulate(client, { simulate: { sender, msgs, data } });

  return await client.broadcast({
    sender,
    credential,
    data,
    msgs,
    gasLimit,
  });
}
