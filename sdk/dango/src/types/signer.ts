import type { Signer as GrugSigner, Prettify } from "@left-curve/types";
import type { Credential } from "./credential.js";
import type { KeyHash } from "./key.js";
import type { Metadata } from "./metadata.js";

export type Signer = Prettify<
  GrugSigner<Metadata, Credential> & {
    getKeyHash(): Promise<KeyHash>;
  }
>;
