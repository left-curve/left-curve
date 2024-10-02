import { keccak_256 as nobleKeccak256 } from "@noble/hashes/sha3";
import { sha256 as nobleSha256 } from "@noble/hashes/sha256";
import { sha512 as nobleSha512 } from "@noble/hashes/sha512";

import type { HashFunction } from "./hash";

export class Keccak256 implements HashFunction {
  readonly blockSize = 136;
  #impl = nobleKeccak256.create();

  public constructor(firstData?: Uint8Array) {
    if (firstData) {
      this.update(firstData);
    }
  }

  public update(data: Uint8Array): Keccak256 {
    this.#impl.update(data);
    return this;
  }

  public digest(): Uint8Array {
    return this.#impl.digest();
  }
}

/** Convenience function equivalent to `new Keccak256(data).digest()` */
export function keccak256(data: Uint8Array): Uint8Array {
  return new Keccak256(data).digest();
}

export class Sha256 implements HashFunction {
  readonly blockSize = 512 / 8;

  #impl = nobleSha256.create();

  public constructor(firstData?: Uint8Array) {
    if (firstData) {
      this.update(firstData);
    }
  }

  public update(data: Uint8Array): Sha256 {
    this.#impl.update(data);
    return this;
  }

  public digest(): Uint8Array {
    return this.#impl.digest();
  }
}

/** Convenience function equivalent to `new Sha256(data).digest()` */
export function sha256(data: Uint8Array): Uint8Array {
  return new Sha256(data).digest();
}

export class Sha512 implements HashFunction {
  public readonly blockSize = 1024 / 8;

  #impl = nobleSha512.create();

  public constructor(firstData?: Uint8Array) {
    if (firstData) {
      this.update(firstData);
    }
  }

  public update(data: Uint8Array): Sha512 {
    this.#impl.update(data);
    return this;
  }

  public digest(): Uint8Array {
    return this.#impl.digest();
  }
}

/** Convenience function equivalent to `new Sha512(data).digest()` */
export function sha512(data: Uint8Array): Uint8Array {
  return new Sha512(data).digest();
}
