import { describe, expect, test } from "vitest";
import { decodeHex, deriveAddress, encodeUtf8 } from ".";

describe("deriving addresses", () => {
  test.each([
    [
      "0972f93fdefd6d18e197ec8d869e9d752432dab29cbb1a4ceb8189793c0c2e8c",
      "0x0a367b92cf0b037dfd89960ee832d56f7fc151681bb41e53690e776f5786998a",
      "account-factory",
      "0x038c6eef147ab94bd38545f9d33e123ffabbab9345c71f2bf31d89c608642289",
    ],
    [
      "f737ee2f533951cc92792115835d54d1a14a3e7995262e6444048e225bf02890",
      "0x0a367b92cf0b037dfd89960ee832d56f7fc151681bb41e53690e776f5786998a",
      "bank",
      "0xecd3d63044b62571eb98dea97bd62142bb7b32b9ab590ccaffa2b50134b19db2"
    ],
    [
      "afaf9da85a987f9f4825337964c971b041ac89194a789fbfb0af85dadcdbf822",
      "0x0a367b92cf0b037dfd89960ee832d56f7fc151681bb41e53690e776f5786998a",
      "cron",
      "0x0e7ee9c971830a350b71555a334b188067fed6206414c48e1faa7a7c6a612a9f"
    ],
  ])("code hash = %s, deployer = %s, salt = %s", (codeHash, deployer, salt, address) => {
    const derivedAddress = deriveAddress(deployer, decodeHex(codeHash), encodeUtf8(salt));
    expect(derivedAddress).toStrictEqual(address);
  })
})
