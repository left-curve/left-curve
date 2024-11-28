import { decodeHex, encodeHex, encodeUtf8, isHex } from "@left-curve/encoding";
import type { EthPersonalMessage, Hex } from "@left-curve/types";
import { keccak256 } from "../sha.js";

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
