import { type KeyPair, Secp256k1, ethHashMessage, keccak256 } from "@left-curve/sdk/crypto";
import { encodeEndian32, encodeHex, encodeUtf8 } from "@left-curve/sdk/encoding";
import { Addr32 } from "./addr32.js";
import { Metadata } from "./multisig.js";

import type { IncrementalMerkleTree } from "./merkletree.js";

const HYPERLANE_DOMAIN_KEY = "HYPERLANE";
const MOCK_REMOTE_MERKLE_TREE = new Uint8Array(32);

type MockValidators = Array<{
  address: string;
  secret: KeyPair;
}>;

export function mockValidatorSet(size: number) {
  const validators = Array.from({ length: size }, () => {
    const secret = Secp256k1.makeKeyPair();

    const address = encodeHex(keccak256(secret.getPublicKey(false).slice(1)).slice(12));

    return {
      address,
      secret,
    };
  });

  return validators;
}

export function mockValidatorSign(
  validators: MockValidators,
  merkleTree: IncrementalMerkleTree,
  messageId: Uint8Array,
  originDomain: number,
): Metadata {
  merkleTree.insert(messageId);

  const merkleRoot = merkleTree.root();
  const merkleIndex = merkleTree.count() - 1n;

  const mHash = ethHashMessage(
    multisigHash(
      domainHash(originDomain, MOCK_REMOTE_MERKLE_TREE, HYPERLANE_DOMAIN_KEY),
      merkleRoot,
      Number(merkleIndex),
      messageId,
    ),
    false,
  );

  const signatures = validators.map(({ secret }) => {
    const signature = secret.createSignature(mHash, true);
    return new Uint8Array([...signature.slice(0, -1), signature[signature.length - 1] + 27]);
  });

  return Metadata.from({
    merkleIndex: Number(merkleIndex),
    merkleRoot,
    originMerkleTree: Addr32.decode(MOCK_REMOTE_MERKLE_TREE),
    signatures: signatures.sort((a, b) => {
      for (let i = 0; i < a.length; i++) {
        if (a[i] < b[i]) {
          return -1;
        }
        if (a[i] > b[i]) {
          return 1;
        }
      }
      return 1;
    }),
  });
}

export function multisigHash(
  domainHash: Uint8Array,
  merkleRoot: Uint8Array,
  merkleIndex: number,
  messageId: Uint8Array,
): Uint8Array {
  const bytes: number[] = [];
  bytes.push(...domainHash);
  bytes.push(...merkleRoot);
  bytes.push(...encodeEndian32(Number(merkleIndex)));
  bytes.push(...messageId);
  return keccak256(new Uint8Array(bytes));
}

export function domainHash(domain: number, address: Uint8Array, key: string) {
  let offset = 0;
  const buff = new Uint8Array(36 + key.length);
  buff.set(encodeEndian32(domain), offset);
  offset += 4;
  buff.set(address, offset);
  offset += 32;
  buff.set(encodeUtf8(key), offset);
  return keccak256(buff);
}
