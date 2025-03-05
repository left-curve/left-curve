import { decodeHex, encodeHex, encodeUtf8, isHex } from "../../encoding/index.js";
import type { Hex } from "../../types/index.js";
import { keccak256 } from "../sha.js";

export type EthPersonalMessage = Hex | string | Uint8Array;

const presignMessagePrefix = "\x19Ethereum Signed Message:\n";

export function ethHashMessage<T extends boolean = true>(
  _message_: EthPersonalMessage,
  hex: T = true as T,
): T extends true ? Hex : Uint8Array {
  const message = (() => {
    if (_message_ instanceof Uint8Array) return _message_;
    return isHex(_message_) ? decodeHex(_message_) : encodeUtf8(_message_);
  })();

  const messageHash = new Uint8Array([
    ...encodeUtf8(presignMessagePrefix),
    ...encodeUtf8(String(message.length)),
    ...message,
  ]);

  return (hex ? encodeHex(keccak256(messageHash)) : keccak256(messageHash)) as T extends true
    ? Hex
    : Uint8Array;
}
