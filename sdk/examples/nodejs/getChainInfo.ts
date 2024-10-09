import { http, createPublicClient } from "@leftcurve/sdk";
import { devnet } from "@leftcurve/sdk/chains";

async function getChainInfo() {
  const client = createPublicClient({
    chain: devnet, // Its optional
    transport: http(devnet.rpcUrls.default.http.at(0)),
  });

  const chainInfo = await client.getChainInfo();

  console.log(chainInfo);
}

getChainInfo().catch(console.error);
