import { type WebAuthnData, parseAsn1Signature } from "./signature.js";

export type VerifyParameters = {
  publicKey: Uint8Array;
  signature: Uint8Array;
  webauthn: WebAuthnData;
};

export async function verifyWebAuthnSignature(parameters: VerifyParameters): Promise<boolean> {
  const { webauthn, publicKey, signature } = parameters;

  const digestClientJSON = new Uint8Array(
    await crypto.subtle.digest("SHA-256", webauthn.clientDataJSON),
  );

  const signedData = new Uint8Array(webauthn.authenticatorData.length + digestClientJSON.length);
  signedData.set(webauthn.authenticatorData);
  signedData.set(digestClientJSON, webauthn.authenticatorData.length);

  const key = await crypto.subtle.importKey(
    "raw",
    publicKey,
    { name: "ECDSA", namedCurve: "P-256" },
    true,
    ["verify"],
  );

  const verified = await crypto.subtle.verify(
    { name: "ECDSA", hash: { name: "SHA-256" } },
    key,
    parseAsn1Signature(signature),
    signedData,
  );

  return verified;
}
