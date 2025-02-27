import { type KeyPair, Secp256k1, domainHash, keccak256, multisigHash } from "../crypto/index.js";
import { encodeHex } from "../encoding/index.js";
import { incrementalMerkleTree } from "./merkletree.js";

const HYPERLANE_DOMAIN_KEY = "HYPERLANE";
// TODO: remote merkle tree
const MOCK_REMOTE_MERKLE_TREE = new Uint8Array();

type Validator = {
  address: string;
  secret: KeyPair;
};

export const mockValidatorSet = (size: number) => {
  const validators: Validator[] = Array.from({ length: size }, () => {
    const secret = Secp256k1.makeKeyPair();

    const address = encodeHex(keccak256(secret.getPublicKey(false).slice(1)).slice(12));

    return {
      address,
      secret,
    };
  });

  const merkleTree = incrementalMerkleTree();

  let nonce = 0;

  const nextNonce = () => {
    nonce += 1;
    return nonce;
  };

  const sign = (messageId: Uint8Array, originDomain: number) => {
    merkleTree.insert(messageId);

    const merkleRoot = merkleTree.root();
    const merkleIndex = merkleTree.count() - 1n;

    const mHash = multisigHash(
      domainHash(originDomain, MOCK_REMOTE_MERKLE_TREE, HYPERLANE_DOMAIN_KEY),
      merkleRoot,
      Number(merkleIndex),
      messageId,
    );

    const signatures = validators.map(({ secret }) => {
      const signature = secret.createSignature(mHash, true);
      return new Uint8Array([...signature.slice(0, -1), signature[signature.length - 1] + 27]);
    });

    return {
      originMerkleTree: MOCK_REMOTE_MERKLE_TREE,
      merkleRoot,
      merkleIndex,
      signatures,
    };
  };

  return {
    nextNonce,
    sign,
    validators,
    nonce,
    merkleTree,
  };
};
