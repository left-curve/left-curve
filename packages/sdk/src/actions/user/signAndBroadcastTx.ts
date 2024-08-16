import type { Account, Chain, Client, Hex, Message, Transport } from "@leftcurve/types";
import { getAccountState } from "../public/getAccountState";
import { getChainInfo } from "../public/getChainInfo";
import { simulate } from "../public/simulate";

export type SignAndBroadcastTxParameters = {
  sender: string;
  msgs: Message[];
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
  const { msgs, sender } = parameters;
  let chainId = client.chain?.id;

  if (!chainId) {
    const response = await getChainInfo(client, {});
    chainId = response.chainId;
  }

  const { sequence } = await getAccountState(client, { address: sender });

  const { credential, data } = await client.account.signTx(msgs, chainId, sequence || 0);
  const { gasLimit } = await simulate(client, { simulate: { sender, msgs } });

  return await client.broadcast({ sender, credential, data, msgs, gasLimit });
}
