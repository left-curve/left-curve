import { http, createPublicClient } from "@leftcurve/sdk";
import { localhost } from "@leftcurve/sdk/chains";
import type { Address } from "@leftcurve/types";

async function getContractsInfo() {
  const client = createPublicClient({
    chain: localhost, // Its optional
    transport: http("http://localhost:26657"),
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
