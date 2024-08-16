import { decodeHex } from "@leftcurve/encoding";
import type { AbstractSigner, Account, Address, Message, Metadata } from "@leftcurve/types";
import { predictAddress } from "../actions";
import { createAccountSalt } from "./salt";

export function toAccount({
  username,
  signer,
}: { username: string; signer: AbstractSigner }): Account {
  async function computeAddress(
    username: string,
    factoryAddr: Address,
    accountTypeCodeHash: string,
  ): Promise<Address> {
    const keyId = await signer.getKeyId();

    return predictAddress({
      deployer: factoryAddr,
      codeHash: decodeHex(accountTypeCodeHash),
      salt: createAccountSalt(username, keyId, 0),
    });
  }

  async function signTx(msgs: Message[], chainId: string, sequence: number) {
    const credential = await signer.signTx(msgs, chainId, sequence);
    const data: Metadata = { username, keyId: await signer.getKeyId(), sequence };

    return { credential, data };
  }

  return {
    username,
    computeAddress,
    getKeyId: () => signer.getKeyId(),
    signTx,
  };
}
