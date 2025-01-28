import { decodeHex, encodeHex, encodeUtf8, isHex } from "../../encoding/index.js";
import type { Hex } from "../../types/index.js";
import { keccak256 } from "../sha.js";

export type EthPersonalMessage = Hex | string | Uint8Array;

const presignMessagePrefix = "\x19Ethereum Signed Message:\n";

export function ethHashMessage(_message_: EthPersonalMessage): Hex {
  const message = (() => {
    if (_message_ instanceof Uint8Array) return _message_;
    return isHex(_message_) ? decodeHex(_message_) : encodeUtf8(_message_);
  })();

  const messageHash = new Uint8Array([
    ...encodeUtf8(presignMessagePrefix),
    ...encodeUtf8(String(message.length)),
    ...message,
  ]);

  return encodeHex(keccak256(messageHash));
}
