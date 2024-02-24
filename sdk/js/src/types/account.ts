import type { Hash } from ".";

export type PublicKey = { secp256k1: string } | { secp256r1: string };

export type AccountFactoryExecuteMsg = {
  registerAccount?: MsgRegisterAccount;
};

export type MsgRegisterAccount = {
  codeHash: Hash;
  publicKey: PublicKey;
};

export type AccountStateResponse = {
  publicKey: PublicKey;
  sequence: number;
};
