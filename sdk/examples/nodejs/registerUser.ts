import { Secp256k1 } from "@leftcurve/crypto";
import { encodeBase64 } from "@leftcurve/encoding";
import { http, computeAddress, createAccountSalt, createUserClient } from "@leftcurve/sdk";
import { PrivateKeySigner } from "@leftcurve/sdk/signers";
import { AccountType, type Address } from "@leftcurve/types";

async function registerUser() {
  if (!process.env.MNEMONIC) throw new Error("Please set the MNEMONIC environment variable");

  // Instantiate the user client
  const userClient = createUserClient({
    username: "owner",
    signer: PrivateKeySigner.fromMnemonic(process.env.MNEMONIC),
    transport: http("http://localhost:26657"),
  });

  const factoryAddr = await userClient.getAppConfig<Address>({ key: "account_factory" });
  const ibcTransferAddr = await userClient.getAppConfig<Address>({ key: "ibc_transfer" });
  const accountCodeHash = await userClient.getAccountTypeCodeHash({
    accountType: AccountType.Spot,
  });

  // User key pair
  const userKeyPair = new Secp256k1(
    new Uint8Array([
      39, 160, 39, 141, 203, 134, 170, 45, 53, 25, 22, 240, 57, 236, 126, 22, 4, 53, 86, 205, 147,
      84, 192, 4, 133, 175, 248, 255, 72, 14, 183, 97,
    ]),
  );

  // create user signer
  const userSigner = PrivateKeySigner.fromPrivateKey(userKeyPair.privateKey);

  const username = "random";
  const userKey = { secp256k1: encodeBase64(userKeyPair.getPublicKey()) };

  // create address and compute new account address
  const salt = createAccountSalt(username, 1, userKey);
  const userAddress = computeAddress({ deployer: factoryAddr, codeHash: accountCodeHash, salt });

  // Send funds to ibc-transfer contract
  await userClient.execute({
    contract: ibcTransferAddr,
    sender: "0x570f0f3f50024058b966e4a7913564be968ede7a",
    msg: {
      receive_transfer: {
        recipient: userAddress,
      },
    },
    funds: { uusdc: "100" },
  });

  // Register user
  await userClient.registerUser({
    keyHash: await userSigner.getKeyHash(),
    key: userKey,
    username: username,
  });

  const balance = await userClient.getBalance({ address: userAddress, denom: "uusdc" });
  console.log(balance);
}

registerUser().catch(console.error);
