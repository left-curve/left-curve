import type { KVNamespace, Request } from "@cloudflare/workers-types";
import {
  PrivateKeySigner,
  createSignerClient,
  devnet,
  graphql,
  isValidAddress,
} from "@left-curve/dango";
import { Secp256k1, keccak256 } from "@left-curve/dango/crypto";
import { decodeHex, deserializeJson, encodeHex, serializeJson } from "@left-curve/dango/encoding";
import {
  Addr32,
  DANGO_DOMAIN,
  IncrementalMerkleTree,
  MAILBOX_VERSION,
  Message,
  TokenMessage,
  mockValidatorSign,
} from "@left-curve/dango/hyperlane";

import type { Address, AppConfig } from "@left-curve/dango/types";

const MOCK_REMOTE_DOMAIN = 123;

interface Env {
  WARP_KV: KVNamespace;
  MOCK_VALIDATORS: string;
  MNEMONIC: string;
  USERNAME: string;
  MINT_AMOUNT: number;
}

const headers = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
};

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (request.method === "GET") return new Response("Ok", { headers, status: 200 });

    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
          "Access-Control-Allow-Headers":
            request.headers.get("Access-Control-Request-Headers") || "*",
          "Access-Control-Max-Age": "86400",
        },
      });
    }
    const client = createSignerClient({
      chain: devnet,
      username: env.USERNAME,
      signer: PrivateKeySigner.fromMnemonic(env.MNEMONIC),
      transport: graphql(devnet.urls.indexer),
    });
    const { addresses } = await client.getAppConfig<AppConfig>();

    const validators = deserializeJson<Array<{ address: string; secret: Uint8Array }>>(
      env.MOCK_VALIDATORS,
    ).map((v) => ({ ...v, secret: new Secp256k1(v.secret) }));

    const { address: userAddr } = await request.json<{ address: Address }>();

    if (!isValidAddress(userAddr)) {
      return new Response("error: invalid address", { headers, status: 400 });
    }

    const merkleeTreeConfig = await env.WARP_KV.get("merkle_tree", "text");
    const merkleTree = merkleeTreeConfig
      ? IncrementalMerkleTree.from(deserializeJson(merkleeTreeConfig))
      : IncrementalMerkleTree.create();

    let currentNonce = Number((await env.WARP_KV.get("nonce")) || "0");

    const messages = Array.from({ length: 5 }, (_, i) => {
      const message = Message.from({
        version: MAILBOX_VERSION,
        originDomain: MOCK_REMOTE_DOMAIN,
        destinationDomain: DANGO_DOMAIN,
        nonce: currentNonce,
        sender: Addr32.decode(
          decodeHex(`000000000000000000000000000000000000000000000000000000000000000${i}`),
        ),
        recipient: Addr32.from(addresses.warp),
        body: TokenMessage.from({
          recipient: Addr32.from(userAddr),
          amount: env.MINT_AMOUNT.toString(),
          metadata: new Uint8Array(0),
        }).encode(),
      }).encode();

      const messageId = keccak256(message);
      const metadata = mockValidatorSign(
        validators,
        merkleTree,
        messageId,
        MOCK_REMOTE_DOMAIN,
      ).encode();
      currentNonce += 1;
      return {
        contract: addresses.hyperlane.mailbox,
        msg: {
          process: {
            raw_message: encodeHex(message),
            raw_metadata: encodeHex(metadata),
          },
        },
      };
    });

    await env.WARP_KV.put("merkle_tree", serializeJson(merkleTree.save()));

    const accounts = await client.getAccountsByUsername({ username: env.USERNAME });
    const sender = Object.keys(accounts)[0] as Address;

    const response = await client.execute({
      sender,
      execute: messages,
    });

    await env.WARP_KV.put("nonce", (currentNonce + 1).toString());

    if (response.code !== 0)
      return new Response("error: the tx went wrong", { headers, status: 500 });

    return new Response("success", { headers, status: 200 });
  },
};
