import type { Account, Chain, Client, Hex, Message, Transport } from "@leftcurve/types";
import { getAccountState } from "../public/getAccountState";
import { getChainInfo } from "../public/getChainInfo";

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

  const accountState = await getAccountState(client, { address: sender }).catch(() => ({
    sequence: 0,
  }));

  const { credential, data } = await client.account.signTx(msgs, sender, chainId, accountState);
  // TODO: get gas limit from chain
  return await client.broadcast({ sender, credential, data, msgs, gasLimit: 10_000_000 });
}
