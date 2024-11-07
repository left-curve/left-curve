import { http, createPublicClient } from "@leftcurve/sdk";
import { devnet } from "@leftcurve/sdk/chains";
import type { Address } from "@leftcurve/types";

async function getContractsInfo() {
  const client = createPublicClient({
    chain: devnet, // Its optional
    transport: http(devnet.rpcUrls.default.http.at(0)),
  });

  const contractsInfo = await client.getContractsInfo();

  console.log(contractsInfo);

  const contractAddresses = Object.keys(contractsInfo);

  if (contractAddresses.length) {
    const contractInfo = await client.getContractInfo({
      address: contractAddresses[0] as Address,
    });

    console.log(contractInfo);
  }
}

getContractsInfo().catch(console.error);
