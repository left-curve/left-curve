import { encodeBase64, encodeHex } from "@leftcurve/encoding";
import type { Account, Chain, Client, Hex, PublicKey, Transport } from "@leftcurve/types";

export type CreateAccountParameters = {
  username: string;
  keyId: Hex;
  pubKey: PublicKey;
  accountType: "spot" | "margin";
};

export type CreateAccountReturnType = Promise<Hex>;

export async function createAccount<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: CreateAccountParameters,
): CreateAccountReturnType {
  const { username, keyId, pubKey, accountType } = parameters;

  const [keyType, key] = Object.entries(pubKey)[0];

  const tx = {
    // TODO: query for account factory
    sender: "0x685e562c38323882918ae8518ea213c092e193e0",
    msgs: [],
    // TODO: query for gas limit
    gasLimit: 1_000_000,
    data: {
      username,
      keyId: typeof keyId === "string" ? keyId : encodeHex(keyId),
      key: { [keyType]: typeof key === "string" ? key : encodeBase64(key) },
      accountType,
    },
    credential: null,
  };

  return await client.broadcast(tx);
}
