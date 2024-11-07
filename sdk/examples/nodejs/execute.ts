import { http, createUserClient } from "@leftcurve/sdk";
import { devnet } from "@leftcurve/sdk/chains";
import { PrivateKeySigner } from "@leftcurve/sdk/signers";

async function execute() {
  const client = createUserClient({
    chain: devnet, // Its optional
    username: "owner",
    signer: PrivateKeySigner.fromRandomKey(),
    transport: http(devnet.rpcUrls.default.http.at(0)),
  });

  const response = await client.execute({ contract: "0x", msg: {}, sender: "0x" });

  console.log(response);
}

execute().catch(console.error);
