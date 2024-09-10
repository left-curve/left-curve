import { requestWebAuthnSignature, sha256 } from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8, serialize } from "@leftcurve/encoding";

import type { KeyHash, SignDoc, Signer } from "@leftcurve/types";
import { createKeyHash } from "../accounts";

export class WebauthnSigner implements Signer {
  async getKeyHash(): Promise<KeyHash> {
    const { credentialId } = await requestWebAuthnSignature({
      challenge: crypto.getRandomValues(new Uint8Array(32)),
      rpId: window.location.hostname,
      userVerification: "preferred",
    });
    return createKeyHash({ credentialId });
  }

  async signTx(signDoc: SignDoc) {
    const { msgs, chainId, sequence } = signDoc;
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
