import { requestWebAuthnSignature } from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8 } from "@leftcurve/encoding";
import { createSignBytes } from "@leftcurve/types";

import type { AbstractSigner } from "@leftcurve/types";
import type { Message, Tx } from "@leftcurve/types";

export class WebauthnSigner implements AbstractSigner {
  async getKeyId(): Promise<string> {
    const { credentialId } = await requestWebAuthnSignature({
      challenge: crypto.getRandomValues(new Uint8Array(32)),
      rpId: window.location.hostname,
      userVerification: "preferred",
    });
    return credentialId;
  }

  async signTx(
    msgs: Message[],
    sender: string,
    chainId: string,
    accountState: { sequence: number },
  ): Promise<Tx> {
    const { sequence } = accountState;
    const tx = createSignBytes(msgs, sender, chainId, sequence);

    const { signature, webauthn } = await requestWebAuthnSignature({
      challenge: tx,
      rpId: window.location.hostname,
      userVerification: "preferred",
    });

    const credential = encodeUtf8(
      JSON.stringify({
        signature,
        webauthn,
      }),
    );

    return {
      sender,
      msgs,
      credential: encodeBase64(credential),
    };
  }
}
