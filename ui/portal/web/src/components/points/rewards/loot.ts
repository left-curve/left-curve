import type { HuntedLoot } from "@left-curve/store";

export type NftRarity = "common" | "uncommon" | "rare" | "epic" | "legendary" | "mythic";

export type LootDisplay = {
  id: string;
  kind: "nft" | "hunted";
  label: string;
  frameSrc: string;
};

const NFT: Record<NftRarity, LootDisplay> = {
  common: {
    id: "common",
    kind: "nft",
    label: "Common",
    frameSrc: "/images/points/nft/frame-common.png",
  },
  uncommon: {
    id: "uncommon",
    kind: "nft",
    label: "Uncommon",
    frameSrc: "/images/points/nft/frame-uncommon.png",
  },
  rare: { id: "rare", kind: "nft", label: "Rare", frameSrc: "/images/points/nft/frame-rare.png" },
  epic: { id: "epic", kind: "nft", label: "Epic", frameSrc: "/images/points/nft/frame-epic.png" },
  legendary: {
    id: "legendary",
    kind: "nft",
    label: "Legendary",
    frameSrc: "/images/points/nft/frame-legendary.png",
  },
  mythic: {
    id: "mythic",
    kind: "nft",
    label: "Mythic",
    frameSrc: "/images/points/nft/frame-mythic.png",
  },
};

const HUNTED: Record<HuntedLoot, Omit<LootDisplay, "id" | "label">> = {
  bronze_shell: { kind: "hunted", frameSrc: "/images/points/boost/frame-bronze.png" },
  silver_shell: { kind: "hunted", frameSrc: "/images/points/boost/frame-silver.png" },
  golden_shell: { kind: "hunted", frameSrc: "/images/points/boost/frame-golden.png" },
  pearl_dango: { kind: "hunted", frameSrc: "/images/points/boost/frame-pearl.png" },
};

export const NFT_RARITY_ORDER: readonly NftRarity[] = [
  "common",
  "uncommon",
  "rare",
  "epic",
  "legendary",
  "mythic",
] as const;

export const HUNTED_ORDER: readonly HuntedLoot[] = [
  "bronze_shell",
  "silver_shell",
  "golden_shell",
  "pearl_dango",
] as const;

export const ALL_NFT_DISPLAYS: readonly LootDisplay[] = NFT_RARITY_ORDER.map((r) => NFT[r]);

export function nftDisplay(rarity: NftRarity): LootDisplay {
  return NFT[rarity];
}

export function huntedDisplay(loot: HuntedLoot, multiplier: string): LootDisplay {
  return {
    id: `hunted-${loot}`,
    kind: "hunted",
    label: `${multiplier}x Boost`,
    frameSrc: HUNTED[loot].frameSrc,
  };
}
