import { encodeBase64, serialize } from "@left-curve/sdk/encoding";
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
  client: DangoClient<transport, Signer>,
  parameters: BroadcastTxSyncParameters,
): BroadcastTxSyncReturnType {
  const { tx } = parameters;

  const result = await client.request({
    method: "broadcast_tx_sync",
    params: {
      tx: encodeBase64(serialize(tx)),
    },
  });

  const { code, codespace, log } = result;

  if (code === 0) return result;

  throw new Error(`failed to broadcast tx! codespace: ${codespace}, code: ${code}, log: ${log}`);
}
