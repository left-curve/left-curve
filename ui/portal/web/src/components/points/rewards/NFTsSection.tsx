import type React from "react";
import { NFTCard, type NFTRarity } from "./NFTCard";

type NFTItem = {
  rarity: NFTRarity;
  quantity: number;
  imageSrc: string;
};

type NFTsSectionProps = {
  nfts?: NFTItem[];
};

const defaultNFTs: NFTItem[] = [
  {
    rarity: "common",
    quantity: 4,
    imageSrc: "https://www.figma.com/api/mcp/asset/c3b5358b-c2b3-4bc0-a1d6-30117c53b423",
  },
  {
    rarity: "uncommon",
    quantity: 2,
    imageSrc: "https://www.figma.com/api/mcp/asset/9680ed08-69ef-471f-83a5-813846101610",
  },
  {
    rarity: "epic",
    quantity: 2,
    imageSrc: "https://www.figma.com/api/mcp/asset/fe211f83-4040-4023-ae76-da8967f68d53",
  },
  {
    rarity: "golden",
    quantity: 2,
    imageSrc: "https://www.figma.com/api/mcp/asset/21afd773-9bb9-4cd4-a30c-dc9a356d0708",
  },
  {
    rarity: "legendary",
    quantity: 2,
    imageSrc: "https://www.figma.com/api/mcp/asset/e21cd2e1-7549-4b53-8916-3d1713bb5699",
  },
  {
    rarity: "rare",
    quantity: 2,
    imageSrc: "https://www.figma.com/api/mcp/asset/555ebcef-e6f2-490b-9c44-b72b36dad681",
  },
];

export const NFTsSection: React.FC<NFTsSectionProps> = ({ nfts = defaultNFTs }) => {
  return (
    <div className="flex flex-col gap-3">
      <p className="h3-bold text-ink-primary-900">My NFTs</p>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
        {nfts.map((nft) => (
          <NFTCard
            key={nft.rarity}
            rarity={nft.rarity}
            quantity={nft.quantity}
            imageSrc={nft.imageSrc}
          />
        ))}
      </div>
    </div>
  );
};
