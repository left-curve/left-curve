export type PublicKey = { secp256k1: string } | { secp256r1: string };

export type AccountFactoryExecuteMsg = {
  registerAccount?: MsgRegisterAccount;
};

export type MsgRegisterAccount = {
  codeHash: string;
  publicKey: PublicKey;
};

export type AccountStateResponse = {
  publicKey: PublicKey;
  sequence: number;
};
