import type { Account, Address, Chain, Client, Hex, Message, Transport } from "@leftcurve/types";
import { getAccountSequence } from "../public/getAccountSequence";
import { getChainInfo } from "../public/getChainInfo";
import { simulate } from "../public/simulate";

export type SignAndBroadcastTxParameters = {
  sender: Address;
  msgs: Message[];
  gasLimit?: number;
};

export type SignAndBroadcastTxReturnType = Promise<Hex>;

export async function signAndBroadcastTx<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: SignAndBroadcastTxParameters,
): SignAndBroadcastTxReturnType {
  if (!client.account) throw new Error("client must have an account");
  const { msgs, sender, gasLimit: gas } = parameters;
  let chainId = client.chain?.id;

  if (!chainId) {
    const response = await getChainInfo(client, {});
    chainId = response.chainId;
  }

  const sequence = await getAccountSequence(client, { address: sender }).catch(() => 0);

  const { credential, data } = await client.account.signTx(msgs, chainId, sequence);

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
