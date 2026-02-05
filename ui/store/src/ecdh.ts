/** biome-ignore-all lint/complexity/noStaticOnlyClass: we want to wrap all together in a class */
import { decodeBase64, decodeUtf8, encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";

export class WebCryptoECDH {
  static #subtle = window?.crypto?.subtle;
  static #ecdhParams = { name: "ECDH", namedCurve: "P-256" };
  static #aesParams = { name: "AES-GCM", length: 256 };

  static async generateKeys(): Promise<CryptoKeyPair> {
    return WebCryptoECDH.#subtle.generateKey(WebCryptoECDH.#ecdhParams, true, ["deriveKey"]);
  }

  static async exportKey(key: CryptoKey): Promise<JsonWebKey> {
    return WebCryptoECDH.#subtle.exportKey("jwk", key);
  }

  static async importPublicKey(jwk: JsonWebKey): Promise<CryptoKey> {
    return WebCryptoECDH.#subtle.importKey("jwk", jwk, WebCryptoECDH.#ecdhParams, true, []);
  }

  static async deriveSecret(privateKey: CryptoKey, publicKey: CryptoKey): Promise<CryptoKey> {
    return WebCryptoECDH.#subtle.deriveKey(
      { name: "ECDH", public: publicKey },
      privateKey,
      WebCryptoECDH.#aesParams,
      true,
      ["encrypt", "decrypt"],
    );
  }

  static async encrypt(secretKey: CryptoKey, message: string): Promise<string> {
    const iv = window.crypto.getRandomValues(new Uint8Array(12));

    const encryptedData = await WebCryptoECDH.#subtle.encrypt(
      { name: "AES-GCM", iv: iv },
      secretKey,
      encodeUtf8(message) as Uint8Array<ArrayBuffer>,
    );

    return `${encodeBase64(iv)}:${encodeBase64(new Uint8Array(encryptedData))}`;
  }

  static async decrypt(secretKey: CryptoKey, payload: string): Promise<string> {
    const [iv, encryptedData] = payload.split(":");
    if (!iv || !encryptedData) throw new Error("Invalid payload format");

    const decryptedBuffer = await WebCryptoECDH.#subtle.decrypt(
      { name: "AES-GCM", iv: decodeBase64(iv) as Uint8Array<ArrayBuffer> },
      secretKey,
      decodeBase64(encryptedData) as Uint8Array<ArrayBuffer>,
    );

    return decodeUtf8(decryptedBuffer);
  }
}
