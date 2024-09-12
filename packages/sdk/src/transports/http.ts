import { decodeBase64, encodeBase64, encodeHex, serialize } from "@leftcurve/encoding";
import { httpRpc, mayTransform } from "@leftcurve/utils";

import { UrlRequiredError } from "~/errors/transports";
import { createTransport } from "./createTransport";

import type {
  AbciQueryResponse,
  Hex,
  JsonRpcBatchOptions,
  RpcAbciQueryResponse,
  RpcBroadcastTxSyncResponse,
  Transport,
  Tx,
  UnsignedTx,
} from "@leftcurve/types";

export type HttpTransportConfig = {
  /**
   * Whether to enable Batch JSON-RPC.
   * @link https://www.jsonrpc.org/specification#batch
   */
  batch?: boolean | JsonRpcBatchOptions;
  /** The name of the transport. */
  name?: string;
  /** The headers of the transport. */
  headers?: Record<string, string | string[]>;
  /** The key of the transport. */
  key?: string;
};

export type HttpTransport = Transport<"http">; /**
 * Creates a HTTP transport that connects to a JSON-RPC API.
 * @param url The URL of the JSON-RPC API.
 * @param config {HttpTransportConfig} The configuration of the transport.
 * @returns The HTTP transport.
 */
export function http(_url_?: string | undefined, config: HttpTransportConfig = {}): HttpTransport {
  const { key = "http", name = "HTTP JSON-RPC", batch: _batch_, headers } = config;
  return ({ chain } = {}) => {
    const url = _url_ || chain?.rpcUrls.default.http[0];
    if (!url) throw new UrlRequiredError();

    const batchOptions = typeof _batch_ === "object" ? _batch_ : { maxSize: 20, maxWait: 20 };
    const batch = _batch_ ? batchOptions : undefined;

    const rpcClient = httpRpc(url, headers, batch);

    async function query(
      path: string,
      data: Uint8Array,
      height = 0,
      prove = false,
    ): Promise<AbciQueryResponse> {
      const { error, result } = await rpcClient.request<{ response: RpcAbciQueryResponse }>(
        "abci_query",
        {
          path,
          prove,
          data: encodeHex(data),
          height: String(height),
        },
      );

      if (error) throw new Error(`Request Error: ${error}`);

      const { response } = result;

      if (response.code === 0) {
        return {
          proof: null,
          code: response.code,
          log: response.log,
          key: decodeBase64(response.key ?? ""),
          value: decodeBase64(response.value ?? ""),
          codespace: response.codespace ?? "",
          height: mayTransform(Number.parseInt, response.height),
          index: mayTransform(Number.parseInt, response.index),
          info: response.info ?? "",
        };
      }
      throw new Error(
        `query failed! codespace: ${response.codespace}, code: ${response.code}, log: ${response.log}`,
      );
    }

    async function broadcast(tx: Tx | UnsignedTx): Promise<Hex> {
      const { error, result } = await rpcClient.request<RpcBroadcastTxSyncResponse>(
        "broadcast_tx_sync",
        {
          tx: encodeBase64(serialize(tx)),
        },
      );

      if (error) throw new Error(`Request Error: ${error}`);
      const { code, codespace, hash, log } = result;

      if (code === 0) {
        return hash;
      }

      throw new Error(
        `failed to broadcast tx! codespace: ${codespace}, code: ${code}, log: ${log}`,
      );
    }

    return createTransport(
      {
        key,
        name,
        type: "http",
      },
      {
        query,
        broadcast,
      },
    );
  };
}
