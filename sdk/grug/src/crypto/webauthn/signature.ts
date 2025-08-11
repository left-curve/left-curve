import { decodeBase64Url } from "../../encoding/base64.js";

export type CredentialAssertion = {
  credentialId: string;
  signature: Uint8Array;
  webauthn: WebAuthnData;
};

export type WebAuthnData = {
  authenticatorData: Uint8Array;
  clientDataJSON: Uint8Array;
};

export type CredentialRequestOptionParameters = {
  credentialId?: string | undefined;
  challenge: Uint8Array;
  /**
   * The relying party identifier to use.
   */
  rpId?: PublicKeyCredentialRequestOptions["rpId"] | undefined;
  userVerification?: PublicKeyCredentialRequestOptions["userVerification"] | undefined;
};

export type SignParameters = CredentialRequestOptionParameters & {
  /**
   * Credential request function. Useful for environments that do not support
   * the WebAuthn API natively (i.e. React Native or testing environments).
   *
   * @default window.navigator.credentials.get
   */
  getFn?:
    | ((options?: CredentialRequestOptions | undefined) => Promise<Credential | null>)
    | undefined;
};

/**
 * Signs a hash using a stored credential. If no credential is provided,
 * a prompt will be displayed for the user to select an existing credential
 * that was previously registered.
 *
 * @example
 * ```ts
 * import { credential } from './credential'
 *
 * const signature = await sign({
 *   credentialId: credential.id,
 *   hash: '0x...',
 * })
 * ```
 */
export async function requestWebAuthnSignature(
  parameters: SignParameters,
): Promise<CredentialAssertion> {
  const { getFn = window.navigator.credentials.get.bind(window.navigator.credentials), ...rest } =
    parameters;
  const options = getCredentialSignRequestOptions(rest);
  try {
    const credential = (await getFn(options)) as PublicKeyCredential;
    if (!credential) throw new Error("credential request failed.");
    const response = credential.response as AuthenticatorAssertionResponse;

    return {
      credentialId: credential.id,
      signature: new Uint8Array(response.signature),
      webauthn: {
        authenticatorData: new Uint8Array(response.authenticatorData),
        clientDataJSON: new Uint8Array(response.clientDataJSON),
      },
    };
  } catch (_error) {
    throw new Error("credential request failed.");
  }
}

/**
 * Returns the request options to sign a hash using a stored credential
 * with a P256 public key.
 *
 * @example
 * ```ts
 * const options = getCredentialSignRequestOptions({ hash: '0x...' })
 * const credentials = window.navigator.credentials.get(options)
 * ```
 */
export function getCredentialSignRequestOptions(
  parameters: CredentialRequestOptionParameters,
): CredentialRequestOptions {
  const {
    credentialId,
    challenge,
    rpId = window.location.hostname,
    userVerification = "required",
  } = parameters;
  // const challenge = base64UrlToBytes(bytesToBase64Url(hexToBytes(hash)));
  return {
    publicKey: {
      ...(credentialId
        ? {
            allowCredentials: [
              {
                id: decodeBase64Url(credentialId),
                type: "public-key",
              },
            ],
          }
        : {}),
      challenge,
      rpId,
      userVerification,
    },
  };
}

/**
 * @param signature
 * Parses an ASN.1 signature into a r and s value.
 * @return The signature as a Uint8Array.
 */

export function parseAsn1Signature(signature: Uint8Array): Uint8Array {
  const rStart = signature[4] === 0 ? 5 : 4;
  const rEnd = rStart + 32;
  const sStart = signature[rEnd + 2] === 0 ? rEnd + 3 : rEnd + 2;
  const r = signature.slice(rStart, rEnd);
  const s = signature.slice(sStart);
  return new Uint8Array([...r, ...s]);
}
