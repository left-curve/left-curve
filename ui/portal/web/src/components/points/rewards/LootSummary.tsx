import { Button, twMerge } from "@left-curve/applets-kit";
import { motion } from "framer-motion";
import type React from "react";

type NFTRarity = "common" | "uncommon" | "rare" | "epic" | "legendary" | "mythic";

type LootCount = Record<NFTRarity, number>;

type LootSummaryProps = {
  lootCounts: LootCount;
  onClose: () => void;
};

const NFT_CONFIG: { rarity: NFTRarity; label: string; frameSrc: string }[] = [
  { rarity: "common", label: "Common", frameSrc: "/images/points/nft/frame-common.png" },
  { rarity: "uncommon", label: "Uncommon", frameSrc: "/images/points/nft/frame-uncommon.png" },
  { rarity: "rare", label: "Rare", frameSrc: "/images/points/nft/frame-rare.png" },
  { rarity: "epic", label: "Epic", frameSrc: "/images/points/nft/frame-epic.png" },
  { rarity: "legendary", label: "Legendary", frameSrc: "/images/points/nft/frame-legendary.png" },
  { rarity: "mythic", label: "Mythic", frameSrc: "/images/points/nft/frame-mythic.png" },
];

export const LootSummary: React.FC<LootSummaryProps> = ({ lootCounts, onClose }) => {
  const totalNFTs = Object.values(lootCounts).reduce((sum, count) => sum + count, 0);

  const handleShareToX = () => {
    const text = `I just opened ${totalNFTs} chests and won some NFTs on Dango!`;
    const url = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}`;
    window.open(url, "_blank");
  };
  return (
    <motion.div
      className="bg-[#3a3530] rounded-2xl overflow-hidden shadow-2xl w-[340px] lg:w-[600px] max-h-[90vh] overflow-y-auto"
      initial={{ opacity: 0, scale: 0.9, y: 20 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      transition={{ duration: 0.3, ease: "easeOut" }}
    >
      <div className="flex items-center justify-between px-4 py-3 border-b border-[#4a4540]">
        <div className="w-6" />
        <p className="diatype-m-medium text-white/80">Your Loot</p>
        <button
          onClick={onClose}
          className="text-white/50 hover:text-white transition-colors"
          type="button"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div className="p-4 lg:p-6">
        <div className="grid grid-cols-2 lg:grid-cols-3 gap-3 lg:gap-4">
          {NFT_CONFIG.map((nft, index) => {
            const count = lootCounts[nft.rarity] ?? 0;
            return (
              <motion.div
                key={nft.rarity}
                className="flex flex-col items-center gap-2"
                initial={{ opacity: 0, scale: 0.8, y: 20 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                transition={{ delay: index * 0.05, duration: 0.3, ease: "easeOut" }}
              >
                <div
                  className={twMerge(
                    "w-full aspect-[320/374] rounded-xl overflow-hidden max-w-[140px] lg:max-w-[160px]",
                    count === 0 && "opacity-50",
                  )}
                >
                  <img
                    src={nft.frameSrc}
                    alt={nft.label}
                    crossOrigin="anonymous"
                    className="w-full h-full object-cover"
                  />
                </div>
                <p className="diatype-m-bold text-white">x{count}</p>
              </motion.div>
            );
          })}
        </div>
      </div>

      <div className="px-4 lg:px-6 pb-4 lg:pb-6 flex flex-col gap-3">
        <Button variant="secondary" className="w-full" onClick={onClose}>
          Done
        </Button>
        <button
          onClick={handleShareToX}
          className="flex items-center justify-center gap-2 text-white/50 hover:text-white transition-colors diatype-sm-medium py-2"
          type="button"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
          </svg>
          Share to X
        </button>
      </div>
    </motion.div>
  );
};
