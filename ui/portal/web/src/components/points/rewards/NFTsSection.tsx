import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount } from "@left-curve/store";
import type React from "react";
import { NFTCard, type NFTRarity } from "./NFTCard";

type NFTItem = {
  rarity: NFTRarity;
  quantity: number;
  imageSrc: string;
  frameSrc: string;
};

type NFTsSectionProps = {
  nfts?: NFTItem[];
};

const defaultNFTs: NFTItem[] = [
  {
    rarity: "common",
    quantity: 4,
    imageSrc: "/images/points/nft/common.png",
    frameSrc: "/images/points/nft/frame-common.png",
  },
  {
    rarity: "uncommon",
    quantity: 2,
    imageSrc: "/images/points/nft/uncommon.png",
    frameSrc: "/images/points/nft/frame-uncommon.png",
  },
  {
    rarity: "rare",
    quantity: 2,
    imageSrc: "/images/points/nft/rare.png",
    frameSrc: "/images/points/nft/frame-rare.png",
  },
  {
    rarity: "epic",
    quantity: 2,
    imageSrc: "/images/points/nft/epic.png",
    frameSrc: "/images/points/nft/frame-epic.png",
  },
  {
    rarity: "legendary",
    quantity: 2,
    imageSrc: "/images/points/nft/legendary.png",
    frameSrc: "/images/points/nft/frame-legendary.png",
  },
  {
    rarity: "mythic",
    quantity: 2,
    imageSrc: "/images/points/nft/mythic.png",
    frameSrc: "/images/points/nft/frame-mythic.png",
  },
];

export const NFTsSection: React.FC<NFTsSectionProps> = ({ nfts = defaultNFTs }) => {
  const { isConnected } = useAccount();

  return (
    <div className="flex flex-col gap-3">
      <p className="h4-bold text-ink-primary-900">{m["points.rewards.nfts.title"]()}</p>
      <div className="grid grid-cols-2 lg:flex lg:flex-wrap gap-4 md:gap-8">
        {nfts.map((nft) => (
          <NFTCard
            key={nft.rarity}
            rarity={nft.rarity}
            quantity={nft.quantity}
            imageSrc={nft.imageSrc}
            frameSrc={nft.frameSrc}
            isLocked={!isConnected}
          />
        ))}
      </div>
    </div>
  );
};
