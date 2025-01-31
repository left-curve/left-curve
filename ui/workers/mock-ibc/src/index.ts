import type { Request } from "@cloudflare/workers-types";
import type { Address } from "@left-curve/dango/types";
import type { DangoAppConfigResponse } from "@left-curve/dango/types/dango";
import { http, createSignerClient, isValidAddress } from "@left-curve/sdk";
import { devnet } from "@left-curve/sdk/chains";
import { PrivateKeySigner } from "@left-curve/sdk/signers";

interface Env {
  MNEMONIC: string;
  USERNAME: string;
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
      transport: http(devnet.rpcUrls.default.http[0]),
    });

    const accounts = await client.getAccountsByUsername({ username: env.USERNAME });
    const address = Object.keys(accounts)[0];

    if (!address) {
      return new Response("error: something went wrong internally", { headers, status: 500 });
    }

    const { address: userAddr } = await request.json<{ address: Address }>();

    if (!isValidAddress(userAddr)) {
      return new Response("error: invalid address", { headers, status: 400 });
    }

    const { addresses } = await client.getAppConfig<DangoAppConfigResponse>();

    const response = await client.execute({
      contract: addresses.ibcTransfer,
      sender: address as Address,
      msg: {
        receive_transfer: {
          recipient: userAddr,
        },
      },
      funds: { uusdc: "100000000" },
    });

    if (response.code !== 0)
      return new Response("error: the tx went wrong", { headers, status: 500 });

    // Wait for the transaction to be processed
    await new Promise((resolve) => setTimeout(resolve, 1500));

    return new Response("success", { headers, status: 200 });
  },
};
