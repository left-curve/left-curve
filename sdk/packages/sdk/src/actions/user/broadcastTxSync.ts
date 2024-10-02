import { encodeBase64, serialize } from "@leftcurve/encoding";
import type {
  Chain,
  Client,
  Prettify,
  Signer,
  Transport,
  Tx,
  TxData,
  UnsignedTx,
} from "@leftcurve/types";

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
export async function broadcastTxSync<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(
  client: Client<Transport, chain, signer>,
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
