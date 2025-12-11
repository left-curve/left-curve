import { describe, expect, it } from "vitest";
import { ethHashMessage } from "@left-curve/sdk/crypto";
import { decodeHex, encodeHex } from "@left-curve/sdk/encoding";
import { IncrementalMerkleTree } from "./merkletree.js";

import cases from "../../../../hyperlane/types/testdata/merkle.json" with { type: "json" };

describe("incremental merklee tree", () => {
  it("insertion works", () => {
    for (const case_ of cases) {
      const tree = IncrementalMerkleTree.create();

      for (const leaf of case_.leaves) {
        const leafHash = ethHashMessage(leaf);
        tree.insert(decodeHex(leafHash));
      }

      const root = tree.root();
      const expectedRoot = case_.expectedRoot;
      expect(encodeHex(root, true)).toBe(expectedRoot);
    }
  });
});
