import { http, createPublicClient } from "@leftcurve/sdk";
import { localhost } from "@leftcurve/sdk/chains";

async function getChainInfo() {
  const client = createPublicClient({
    chain: localhost, // Its optional
    transport: http("http://localhost:26657"),
  });

  const chainInfo = await client.getChainInfo();

  console.log(chainInfo);
}

getChainInfo().catch(console.error);
