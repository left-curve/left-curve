import { decode } from "cbor-x";
import { encodeUtf8 } from "../../encoding/utf8.js";
import { sha256 } from "../sha.js";

export type CredentialAttestion = {
  id: string;
  raw: PublicKeyCredential;
  getPublicKey: (compressed?: boolean) => Promise<Uint8Array>;
};

export type CreateCredentialParameters = CredentialOptionParameters & {
  /**
   * Credential creation function. Useful for environments that do not support
   * the WebAuthn API natively (i.e. React Native or testing environments).
   *
   * @default window.navigator.credentials.create
   */
  createFn?:
    | ((options?: CredentialCreationOptions | undefined) => Promise<Credential | null>)
    | undefined;
};

// https://developer.mozilla.org/en-US/docs/Web/API/PublicKeyCredentialCreationOptions
export type CredentialOptionParameters = {
  /**
   * A string specifying the relying party's preference for how the attestation statement
   * (i.e., provision of verifiable evidence of the authenticity of the authenticator and its data)
   * is conveyed during credential creation.
   */
  attestation?: PublicKeyCredentialCreationOptions["attestation"];
  /**
   * An object whose properties are criteria used to filter out the potential authenticators
   * for the credential creation operation.
   */
  authenticatorSelection?: PublicKeyCredentialCreationOptions["authenticatorSelection"];
  /**
   * An ArrayBuffer, TypedArray, or DataView provided by the relying party's server
   * and used as a cryptographic challenge. This value will be signed by the authenticator
   * and the signature will be sent back as part of AuthenticatorAttestationResponse.attestationObject.
   */
  challenge?: PublicKeyCredentialCreationOptions["challenge"];
  /**
   * List of credential IDs to exclude from the creation. This property can be used
   * to prevent creation of a credential if it already exists.
   */
  excludeCredentialIds?: readonly string[];
  /**
   * List of Web Authentication API credentials to use during creation or authentication.
   * Extensions are optional and different browsers may recognize different extensions.
   * Processing extensions is always optional for the client:
   * if a browser does not recognize a given extension, it will just ignore it
   */
  extensions?: PublicKeyCredentialCreationOptions["extensions"];
  /**
   * An object describing the relying party that requested the credential creation.
   */
  rp?: {
    /**
     * A human-readable name for the relying party.
     */
    name: string;
    /**
     * A unique identifier for the relying party.
     */
    id: string;
  };
  /**
   * A numerical hint, in milliseconds, which indicates the time the calling web app is willing to wait for the creation operation to complete.
   * This hint may be overridden by the browser.
   */
  timeout?: number;
  /**
   * An object describing the user account for which the credential is generated.
   */
  user: {
    /**
     * A human-readable name for the user.
     */
    displayName?: string;
    /**
     * A globally unique identifier for the user.
     */
    id?: string;
    /**
     * A human-readable name for the user.
     */
    name: string;
  };
};

/**
 * Challange for credential creation - random 16 bytes
 */
export function createChallenge(): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(16));
}

export async function createWebAuthnCredential(
  params: CreateCredentialParameters,
): Promise<CredentialAttestion> {
  const {
    createFn = window.navigator.credentials.create.bind(window.navigator.credentials),
    ...rest
  } = params;
  const options = getCredentialCreationOptions(rest);
  try {
    const credential = (await createFn(options)) as PublicKeyCredential;
    if (!credential) throw new Error("credential creation failed.");
    const response = (credential as PublicKeyCredential)
      .response as AuthenticatorAttestationResponse;

    async function getPublicKey(compressed = true): Promise<Uint8Array> {
      const publicKey = decodePublicKey(response.attestationObject);
      if (!compressed) return publicKey;
      const { p256 } = await import("@noble/curves/p256");
      return p256.ProjectivePoint.fromHex(publicKey).toRawBytes(true);
    }

    return {
      id: credential.id,
      raw: credential,
      getPublicKey,
    };
  } catch (_error) {
    throw new Error("credential creation failed.");
  }
}

/**
 * Returns the creation options for a P256 WebAuthn Credential with a Passkey authenticator.
 *
 * @example
 * ```ts
 * const options = getCredentialCreationOptions({ name: 'Example' })
 * const credentials = window.navigator.credentials.create(options)
 * ```
 */
export function getCredentialCreationOptions(
  parameters: CredentialOptionParameters & {},
): CredentialCreationOptions {
  const {
    attestation = "none",
    authenticatorSelection = {
      authenticatorAttachment: "platform",
      residentKey: "preferred",
      requireResidentKey: false,
      userVerification: "required",
    },
    challenge = createChallenge(),
    excludeCredentialIds,
    rp = {
      id: window.location.hostname,
      name: window.document.title,
    },
    user,
    extensions,
  } = parameters;

  return {
    publicKey: {
      attestation,
      authenticatorSelection,
      challenge,
      ...(excludeCredentialIds
        ? {
            excludeCredentials: excludeCredentialIds?.map((id) => ({
              id: id, // base64UrlToBytes(id),
              type: "public-key",
            })),
          }
        : {}),
      pubKeyCredParams: [
        {
          type: "public-key",
          alg: -7, // p256
        },
      ],
      rp,
      user: {
        id: user.id ?? sha256(encodeUtf8(user.name)),
        name: user.name,
        displayName: user.displayName ?? user.name,
      },
      extensions,
    },
  } as unknown as CredentialCreationOptions;
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
  const decodedAttestationObj = decode(new Uint8Array(attestation));

  const { authData } = decodedAttestationObj;

  const publicKeyObject = decode(new Uint8Array(authData.slice(-77).buffer));
  // -2: The -2 field describes the x-coordinate of this public key.
  const xPoint = publicKeyObject["-2"];
  // -3: The -3 field describes the y-coordinate of this
  const yPoint = publicKeyObject["-3"];
  return new Uint8Array(["04", ...xPoint, ...yPoint]);
}
