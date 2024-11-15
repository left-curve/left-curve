"use client";

import { motion } from "framer-motion";
import type { PropsWithChildren } from "react";

export const DisplayIntro: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <motion.div
      className="flex items-center justify-between w-full flex-1 flex-col gap-10 md:gap-4 md:pt-8 p-4 md:p-0 text-typography-black-100"
      initial={{ opacity: 0, translateY: 100 }}
      animate={{ opacity: 1, translateY: 0 }}
      exit={{ opacity: 0, translateY: 100 }}
    >
      <h2 className="text-xl">What is an account?</h2>
      <div className="flex flex-col gap-8">
        <div className="flex gap-4 w-full">
          <div className="bg-surface-green-400 rounded-lg h-12 w-12" />
          <div className="flex flex-col flex-1 w-full">
            <p className="text-lg font-bold">Safeguard and manage your digital assets</p>
            <p className="text-gray-500">Securely store and transfer your cryptos and NFT's</p>
          </div>
        </div>
        <div className="flex gap-4 w-full">
          <div className="bg-surface-green-400 rounded-lg h-12 w-12" />
          <div className="flex flex-col flex-1 w-full">
            <p className="text-lg font-bold">Log in with Grug!</p>
            <p className="text-gray-500">Connect your account to access your digital assets</p>
          </div>
        </div>
      </div>
      {children}
    </motion.div>
  );
};
