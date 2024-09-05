import { requestWebAuthnSignature, ripemd160, sha256 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, encodeUtf8, serialize } from "@leftcurve/encoding";

import type { KeyHash, Signer } from "@leftcurve/types";
import type { Message } from "@leftcurve/types";

export class WebauthnSigner implements Signer {
  async getKeyHash(): Promise<KeyHash> {
    const { credentialId } = await requestWebAuthnSignature({
      challenge: crypto.getRandomValues(new Uint8Array(32)),
      rpId: window.location.hostname,
      userVerification: "preferred",
    });
    return encodeHex(ripemd160(encodeUtf8(credentialId))).toUpperCase();
  }

  async signTx(msgs: Message[], chainId: string, sequence: number) {
    const tx = sha256(serialize({ messages: msgs, chainId, sequence }));

    const { signature, webauthn } = await requestWebAuthnSignature({
      challenge: tx,
      rpId: window.location.hostname,
      userVerification: "preferred",
    });

    const webAuthnSignature = encodeUtf8(
      JSON.stringify({
        signature,
        webauthn,
      }),
    );

    const credential = { passkey: encodeBase64(webAuthnSignature) };
    const keyHash = await this.getKeyHash();

    return { credential, keyHash };
  }
}
