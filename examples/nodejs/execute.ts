import { http, createUserClient } from "@leftcurve/sdk";
import { localhost } from "@leftcurve/sdk/chains";
import { PrivateKeySigner } from "@leftcurve/sdk/signers";

async function execute() {
  const client = createUserClient({
    chain: localhost, // Its optional
    signer: PrivateKeySigner.fromRandomKey(),
    transport: http("http://localhost:26657"),
  });

  const response = await client.execute({ contract: "0x", msg: {}, sender: "0x" });

  console.log(response);
}

execute().catch(console.error);
