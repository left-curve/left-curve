import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { AdminOptionKind, Client, SigningKey, encodeHex, encodeUtf8 } from "../src";

const artifactDir = path.resolve(__dirname, "../../../artifacts");
const keystoreDir = path.join(os.homedir(), ".cwcli/keys");
const keystorePassword = "123";

const user = "0xd314da8374f3e890f57fe0c949810a1d8b7d71d576c553aae2e6eb6f9961c721";
const bank = "0xed0c6d9c00ade34a7e8a3ec5e37b195ab29f1b8c12ec68ee43874a792c7c46ed";

async function sleep(seconds: number) {
  return new Promise(resolve => setTimeout(resolve, seconds * 1000));
}

(async function () {
  // load signing key
  const test1 = await SigningKey.fromFile(path.join(keystoreDir, "test1.json"), keystorePassword);
  const signingOpts = {
    sender: user,
    signingKey: test1,
  };

  // create client
  const client = await Client.connect("http://127.0.0.1:26657");

  // store and instantiate token wrapper contract
  const wrapperWasm = fs.readFileSync(path.join(artifactDir, "cw_mock_token_wrapper-aarch64.wasm"));
  const [wrapper, tx1] = await client.storeCodeAndInstantiate(
    wrapperWasm,
    { bank },
    encodeUtf8("wrapper"),
    [],
    AdminOptionKind.SetToNone,
    signingOpts,
  );
  console.log("\nwrapper contract instantiated!");
  console.log("address:", wrapper);
  console.log("txhash:", encodeHex(tx1));

  // wait 2 seconds for tx to settle
  await sleep(2);

  // query the user's balances
  const balancesBefore = await client.queryBalances(user);
  console.log("\nusers balance before wrapping:\n" + JSON.stringify(balancesBefore, null, 2));

  // wrap some tokens
  const tx2 = await client.transfer(
    wrapper,
    [
      {
        denom: "uatom",
        amount: "888",
      },
      {
        denom: "uosmo",
        amount: "999",
      },
    ],
    signingOpts,
  );
  console.log("\ntokens wrapped!");
  console.log("txhash:", encodeHex(tx2));

  // wait 2 seconds for tx to settle
  await sleep(2);

  // query the user's balances again
  const balancesAfter = await client.queryBalances(user);
  console.log("\nuser balances after wrapping\n" + JSON.stringify(balancesAfter, null, 2));
})();
