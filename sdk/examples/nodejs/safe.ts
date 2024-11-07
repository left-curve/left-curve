import { http, createSignerClient } from "@leftcurve/sdk";
import { safeActions } from "@leftcurve/sdk/actions";
import { devnet } from "@leftcurve/sdk/chains";
import { PrivateKeySigner } from "@leftcurve/sdk/signers";

async function execute() {
  const client = createSignerClient({
    chain: devnet, // Its optional
    username: "owner",
    signer: PrivateKeySigner.fromRandomKey(),
    transport: http(devnet.rpcUrls.default.http.at(0)),
  }).extend(safeActions);

  const proposal = client.safeAccountGetProposal({
    address: "0x",
    proposalId: 1,
  });

  console.log(proposal);
}

execute().catch(console.error);
