import { Button, DangoButton, Input, Select, SelectItem } from "@dango/shared";
import { useConfig } from "@leftcurve/react";
import { motion } from "framer-motion";

import type { NativeCoin } from "@leftcurve/types";

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
      <div className="flex flex-col gap-8 items-center text-center w-full">
        <h3 className="text-typography-black-200 font-extrabold text-lg tracking-widest uppercase">
          Transfer Assets
        </h3>
        <p className="text-typography-black-100 text-xl">
          Fund your account with assets from other existing wallets of yours
        </p>
      </div>
      <div className="flex flex-col gap-2 w-full">
        <div className="flex flex-col gap-8 w-full">
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
                    <img
                      src={logoURI}
                      className="w-8 h-8 rounded-full"
                      alt="logo-chain-native-coin"
                    />
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
            <DangoButton color="rose" fullWidth>
              Connect Wallet
            </DangoButton>
            <p className="uppercase text-typography-pink-200 text-xs font-extrabold">
              Powered by IBC/Noble/CCTP
            </p>
          </div>
        </div>
      </div>
    </motion.div>
  );
};
