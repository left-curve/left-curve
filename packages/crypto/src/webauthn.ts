import { fromCbor } from "@leftcurve/encoding";

export type CredentialCreationOptions = {
  timeout?: number;
  domain?: string;
  rp_name?: string;
};

export type CredentialRequestOptions = {
  timeout?: number;
  domain?: string;
};

export type CredentialAttestion = {
  id: string;
  publicKey: Uint8Array;
};

/**
 * Creates a cryptographic key pair, stores the private key in the secure enclave and expose the public key.
 *
 * @returns The public key in raw format and the credential ID.
 */
export async function createWebauthnCredential(credentialName: string, options?: CredentialCreationOptions): Promise<CredentialAttestion> {
  const publicKeyCreationOptions: PublicKeyCredentialCreationOptions = {
    challenge: crypto.getRandomValues(new Uint8Array(32)),
    rp: {
      id: options?.domain ?? window.location.host,
      name: options?.rp_name ?? window.location.host,
    },
    user: {
      id: crypto.getRandomValues(new Uint8Array(32)),
      name: credentialName,
      displayName: credentialName,
    },
    pubKeyCredParams: [
      { alg: -7, type: "public-key" }, // ES256
      { type: "public-key", alg: -257 }, // RS256
    ],
    timeout: options?.timeout ?? 60_000,
  };

  const credential = await navigator.credentials.create({
    publicKey: publicKeyCreationOptions,
  });

  if (!credential) throw new Error("Credential creation failed");

  const response = (credential as PublicKeyCredential).response as AuthenticatorAttestationResponse;

  const publicKey = decodePublicKey(response.attestationObject);

  return {
    publicKey,
    id: credential.id,
  };
}
/**
 * Signs a challenge using the private key stored in the secure enclave.
 *
 * @returns An object containing the signature, the authenticator data and client data.
 */
export async function requestWebauthnSignature(
  challenge: Uint8Array,
  options?: CredentialRequestOptions
): Promise<AuthenticatorAssertionResponse> {
  const publicKeyCredentialRequestOptions: PublicKeyCredentialRequestOptions = {
    rpId: options?.domain ?? window.location.host,
    challenge,
    timeout: options?.timeout ?? 60_000,
  };

  const assertion = await navigator.credentials.get({
    publicKey: publicKeyCredentialRequestOptions,
  });

  if (!assertion) throw new Error("Sign operation failed");

  return (assertion as PublicKeyCredential).response as AuthenticatorAssertionResponse;
}

/**
 *
 *
 * The attestation object is a CBOR encoded object. The CBOR object have the following fields:
 * 1. authData: The authenticator data.
 * 2. fmt: The attestation statement format identifier.
 * 3. attStmt: The attestation statement.
 * https://w3c.github.io/webauthn/#attestation-object
 * @returns The public key in raw format.
 */
function decodePublicKey(attestation: ArrayBuffer): Uint8Array {
  const decodedAttestationObj = fromCbor(new Uint8Array(attestation));

  const { authData } = decodedAttestationObj;

  const publicKeyObject = fromCbor(new Uint8Array(authData.slice(-77).buffer));
  // -2: The -2 field describes the x-coordinate of this public key.
  const xPoint = publicKeyObject["-2"];
  // -3: The -3 field describes the y-coordinate of this
  const yPoint = publicKeyObject["-3"];

  return new Uint8Array(["04", ...xPoint, ...yPoint]);
}
