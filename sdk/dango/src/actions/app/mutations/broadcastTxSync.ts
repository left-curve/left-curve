import type { Prettify, Transport, Tx, TxData, UnsignedTx } from "@left-curve/sdk/types";

import type { DangoClient, Signer } from "../../../types/index.js";

export type BroadcastTxSyncParameters = {
  tx: Tx | UnsignedTx;
};

export type BroadcastTxSyncReturnType = Promise<Prettify<{ hash: Uint8Array } & TxData>>;

/**
 * Broadcasts a transaction synchronously.
 * @param parameters
 * @param parameters.tx The transaction to broadcast.
 * @returns The transaction hash and data.
 */
export async function broadcastTxSync<transport extends Transport>(
  client: DangoClient<transport, undefined | Signer>,
  parameters: BroadcastTxSyncParameters,
): BroadcastTxSyncReturnType {
  const { tx } = parameters;

  return await client.request({
    method: "broadcast",
    params: {
      mode: "sync",
      tx,
    },
  });
}
