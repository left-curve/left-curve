import { http, createUserClient } from "@leftcurve/sdk";
import { safeActions } from "@leftcurve/sdk/actions";
import { localhost } from "@leftcurve/sdk/chains";
import { PrivateKeySigner } from "@leftcurve/sdk/signers";

async function execute() {
  const client = createUserClient({
    chain: localhost, // Its optional
    username: "owner",
    signer: PrivateKeySigner.fromRandomKey(),
    transport: http("http://localhost:26657"),
  }).extend(safeActions);

  const proposal = client.safeAccountGetProposal({
    address: "0x",
    proposalId: 1,
  });

  console.log(proposal);
}

execute().catch(console.error);
