import type { Message, Tx } from "./tx";

export type AbstractSigner<T = unknown> = {
  getKeyId(): Promise<string>;
  signTx(msgs: Message[], sender: string, chainId: string, accountState: T): Promise<Tx>;
};
