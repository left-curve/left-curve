import type React from "react";
import { NFTCard, type NFTRarity } from "./NFTCard";

type NFTItem = {
  rarity: NFTRarity;
  quantity: number;
  imageSrc: string;
};

type NFTsSectionProps = {
  nfts: NFTItem[];
};

export const NFTsSection: React.FC<NFTsSectionProps> = ({ nfts }) => {
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
