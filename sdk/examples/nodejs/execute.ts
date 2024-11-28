import { http, createSignerClient } from "@left-curve/sdk";
import { devnet } from "@left-curve/sdk/chains";
import { PrivateKeySigner } from "@left-curve/sdk/signers";

async function execute() {
  const client = createSignerClient({
    chain: devnet, // Its optional
    username: "owner",
    signer: PrivateKeySigner.fromRandomKey(),
    transport: http(devnet.rpcUrls.default.http.at(0)),
  });

  const response = await client.execute({ contract: "0x", msg: {}, sender: "0x" });

  console.log(response);
}

execute().catch(console.error);
