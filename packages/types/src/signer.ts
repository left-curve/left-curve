import type { Hex } from "./common";
import type { Credential } from "./credential";
import type { Message } from "./tx";

export type AbstractSigner = {
  getKeyId: () => Promise<Hex>;
  signTx: (msgs: Message[], chainId: string, sequence: number) => Promise<Credential>;
};
