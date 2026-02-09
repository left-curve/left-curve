import { Button } from "@left-curve/applets-kit";
import { motion } from "framer-motion";
import type React from "react";
import type { NFTItem } from "./NFTCarousel";

type NFTResultProps = {
  nft: NFTItem;
  onContinue: () => void;
};

export const NFTResult: React.FC<NFTResultProps> = ({ nft, onContinue }) => {
  const handleShareToX = () => {
    const text = `I just won a ${nft.label} NFT on Dango!`;
    const url = `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}`;
    window.open(url, "_blank");
  };

  return (
    <motion.div
      className="bg-[#3a3530] rounded-2xl overflow-hidden shadow-2xl w-[320px] lg:w-[380px]"
      initial={{ opacity: 0, scale: 0.9, y: 20 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      transition={{ duration: 0.3, ease: "easeOut" }}
    >
      <div className="flex items-center justify-between px-4 py-3 border-b border-[#4a4540]">
        <div className="w-6" />
        <p className="diatype-m-medium text-white/80">NFT Card</p>
        <button
          onClick={onContinue}
          className="text-white/50 hover:text-white transition-colors"
          type="button"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div className="p-6 flex flex-col items-center gap-4">
        <motion.div
          className="w-full aspect-square max-w-[240px] lg:max-w-[280px] rounded-xl overflow-hidden bg-[#2a2520] flex items-center justify-center p-4"
          initial={{ scale: 0.8, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          transition={{ delay: 0.1, duration: 0.4, ease: [0.34, 1.56, 0.64, 1] }}
        >
          <img
            src={nft.imageSrc}
            alt={nft.label}
            crossOrigin="anonymous"
            className="w-full h-full object-contain [filter:drop-shadow(0px_4px_40px_rgba(227,189,102,0.5))]"
          />
        </motion.div>
      </div>

      <div className="px-6 pb-6 flex flex-col gap-3">
        <Button
          variant="secondary"
          className="w-full"
          onClick={onContinue}
        >
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
