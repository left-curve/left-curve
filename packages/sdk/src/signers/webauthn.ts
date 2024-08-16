import { requestWebAuthnSignature, sha256 } from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8, serialize } from "@leftcurve/encoding";

import type { AbstractSigner, Credential, Json } from "@leftcurve/types";
import type { Message } from "@leftcurve/types";

export class WebauthnSigner implements AbstractSigner {
  async getKeyId(): Promise<string> {
    const { credentialId } = await requestWebAuthnSignature({
      challenge: crypto.getRandomValues(new Uint8Array(32)),
      rpId: window.location.hostname,
      userVerification: "preferred",
    });
    return credentialId;
  }

  async signTx(msgs: Message[], chainId: string, sequence: number): Promise<Credential> {
    const tx = sha256(serialize({ messages: msgs, chainId, sequence }));

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

    return { passkey: encodeBase64(credential) };
  }
}
