import {
  type KeyPair,
  Secp256k1,
  domainHash,
  ethHashMessage,
  keccak256,
  multisigHash,
} from "../crypto/index.js";
import { encodeHex } from "../encoding/index.js";
import { Addr32 } from "./addr32.js";
import type { IncrementalMerkleTree } from "./merkletree.js";
import { Metadata } from "./multisig.js";

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
