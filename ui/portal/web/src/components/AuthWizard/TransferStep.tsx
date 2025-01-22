import { Button, Input, Select, SelectItem } from "@left-curve/applets-kit";
import { useConfig } from "@left-curve/react";
import { motion } from "framer-motion";

import type { NativeCoin } from "@left-curve/types";

export const TransferStep: React.FC = () => {
  const { chains, coins } = useConfig();
  const chain = chains.at(0)!;
  const chainCoins = coins[chain.id];
  const { logoURI, symbol } = chainCoins[chain.nativeCoin.denom] as NativeCoin;

  return (
    <motion.div
      className="flex flex-col w-full justify-center gap-8"
      initial={{ translateY: -100 }}
      animate={{ translateY: 0 }}
      exit={{ translateY: 100 }}
    >
      <div className="p-3 bg-surface-rose-200 w-full rounded-[20px] flex flex-col gap-6">
        <div className="w-full flex flex-col gap-2">
          <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
            Deposit amount
          </p>
          <Input
            startText="right"
            placeholder="0"
            startContent={
              <div className="flex flex-row items-center gap-2">
                <img src={logoURI} className="w-8 h-8 rounded-full" alt="logo-chain-native-coin" />
                <span className="text-typography-black-100 inline-block">{symbol}</span>
              </div>
            }
          />
        </div>
        <div className="w-full flex flex-col gap-2">
          <p className="font-extrabold text-typography-rose-500 tracking-widest uppercase text-sm">
            select network
          </p>
          <Select placeholder="Choose network" label="network">
            <SelectItem>Ethereum</SelectItem>
            <SelectItem>Solana</SelectItem>
          </Select>
        </div>
      </div>

      <div className="flex flex-col gap-1 w-full items-center justify-center">
        <Button color="rose" fullWidth>
          Connect Wallet
        </Button>
      </div>
    </motion.div>
  );
};
