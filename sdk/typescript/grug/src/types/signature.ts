import type { Hex } from "./encoding.js";

export type SignDoc<Doc> = Doc;

export type SignatureOutcome<signed, credential> = {
  credential: credential;
  signed: signed;
};

export type RawSignature = {
  r: Hex;
  s: Hex;
  v: number;
};
