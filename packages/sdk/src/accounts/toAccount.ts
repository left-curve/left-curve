import type { AbstractSigner, Account, Message, Metadata } from "@leftcurve/types";

export function toAccount({
  username,
  signer,
}: { username: string; signer: AbstractSigner }): Account {
  async function signTx(msgs: Message[], chainId: string, sequence: number) {
    const credential = await signer.signTx(msgs, chainId, sequence);
    const data: Metadata = { keyHash: await signer.getKeyId(), sequence };

    return { credential, data };
  }

  return {
    username,
    getKeyId: () => signer.getKeyId(),
    signTx,
  };
}
