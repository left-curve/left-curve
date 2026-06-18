import { describe, expect, it } from "vitest";
import { ethHashMessage } from "@left-curve/crypto";
import { decodeHex, encodeHex } from "@left-curve/encoding";
import { IncrementalMerkleTree } from "./merkletree.js";

const cases = [
  {
    testName: "no leaves",
    expectedRoot: "0x27ae5ba08d7291c96c8cbddcc148bf48a6d68c7974b94356f53754ef6171d757",
    leaves: [],
  },
  {
    testName: "one leaf",
    expectedRoot: "0x54fea87823728b754368018753f79a24e5f5cacee26c8785f3d33aabfd03372e",
    leaves: ["one"],
  },
  {
    testName: "three leaves",
    expectedRoot: "0x18f2f1646fee335a1eaf5191a8ce58ea772080057d0fda687df59c45e47e6f68",
    leaves: ["one", "two", "three"],
  },
  {
    testName: "forty-two leaves",
    expectedRoot: "0x274d610098d8f109587e97c908cf549d129a14f5bad7eb10d36a427da97be6fc",
    leaves: [
      "bacon",
      "eye",
      "we",
      "ghost",
      "listen",
      "corn",
      "blonde",
      "gutter",
      "sanctuary",
      "seat",
      "generate",
      "twist",
      "waterfall",
      "monster",
      "elbow",
      "flash",
      "arrow",
      "moment",
      "cheat",
      "unity",
      "steak",
      "shelter",
      "camera",
      "album",
      "bread",
      "tease",
      "sentence",
      "tribe",
      "miserable",
      "ridge",
      "guerrilla",
      "inhabitant",
      "suspicion",
      "mosque",
      "printer",
      "land",
      "reliable",
      "circle",
      "first-hand",
      "time",
      "content",
      "management",
    ],
  },
];

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
