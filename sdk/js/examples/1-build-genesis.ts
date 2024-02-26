import * as os from "os";
import * as path from "path";
import { AdminOptionKind, GenesisBuilder, SigningKey, encodeUtf8 } from "../src";

const artifactDir = path.resolve(__dirname, "../../../artifacts");
const keystoreDir = path.join(os.homedir(), ".cwcli/keys");
const keystorePassword = "123";

(async () => {
  // load two pubkeys from the keystore. we will register an account for each of them
  const test1 = await SigningKey.fromFile(path.join(keystoreDir, "test1.json"), keystorePassword);
  const test2 = await SigningKey.fromFile(path.join(keystoreDir, "test2.json"), keystorePassword);

  // create the genesis builder
  const builder = new GenesisBuilder();

  // upload account wasm code
  const accountCodeHash = builder.storeCode(path.join(artifactDir, "cw_account-aarch64.wasm"));

  // store and instantiate account factory contract
  const accountFactory = builder.storeCodeAndInstantiate(
    path.join(artifactDir, "cw_account_factory-aarch64.wasm"),
    {},
    encodeUtf8("account-factory"),
    AdminOptionKind.SetToNone,
  );

  const account1 = builder.registerAccount(accountFactory, accountCodeHash, test1.publicKey());
  const account2 = builder.registerAccount(accountFactory, accountCodeHash, test2.publicKey());

  // store and instantiate and bank contract
  // give account1 some initial balances
  const bank = builder.storeCodeAndInstantiate(
    path.join(artifactDir, "cw_bank-aarch64.wasm"),
    {
      initialBalances: [
        {
          address: account1,
          coins: [
            {
              denom: "uatom",
              amount: "12345",
            },
            {
              denom: "uosmo",
              amount: "23456",
            },
          ],
        },
      ],
    },
    encodeUtf8("bank"),
    AdminOptionKind.SetToNone,
  );

  builder.setConfig({ bank });
  builder.writeToFile();

  console.log("✅ done!");
  console.log("account-factory :", accountFactory);
  console.log("account1        :", account1);
  console.log("account2        :", account2);
  console.log("bank            :", bank);
})();
