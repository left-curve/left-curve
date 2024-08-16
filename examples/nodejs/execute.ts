import { http, createUserClient, toAccount } from "@leftcurve/sdk";
import { localhost } from "@leftcurve/sdk/chains";
import { PrivateKeySigner } from "@leftcurve/sdk/signers";

async function execute() {
  const client = createUserClient({
    chain: localhost, // Its optional
    account: toAccount({ username: "random-user", signer: PrivateKeySigner.fromRandomKey() }),
    transport: http("http://localhost:26657"),
  });

  const response = await client.execute({ contract: "", funds: {}, msg: {}, sender: "" });

  console.log(response);
}

execute().catch(console.error);
