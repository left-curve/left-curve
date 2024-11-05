import { Button, GradientContainer, Input, SearchIcon } from "@dango/shared";
import type React from "react";

export const PoolSelector: React.FC = () => {
  return (
    <GradientContainer className="w-full flex flex-col gap-9">
      <h2 className="font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest mb-1">
        SELECT POOL
      </h2>
      <div className="flex flex-col gap-4 w-full">
        <Input
          classNames={{
            inputWrapper:
              "bg-surface-purple-100 border border-purple-600/40 group-hover:bg-surface-purple-200 text-typography-black-100 px-2 rounded-2xl",
            input: "placeholder:text-typography-black-100/40 text-typography-black-100 text-xl",
          }}
          placeholder="Search tokens"
          startContent={<SearchIcon className="h-6 w-6 text-typography-black-100/40" />}
        />
        <div className="flex flex-col gap-1">
          <div className="px-6 gap-1 grid grid-cols-[1fr_80px_80px_80px] text-end">
            <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest text-start">
              ASSET
            </p>
            <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
              BALANCE
            </p>
            <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
              TVL
            </p>
            <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
              APR
            </p>
          </div>
          <div className="py-4 px-6 items-center gap-1 grid grid-cols-[1fr_80px_80px_80px] text-end bg-surface-rose-100 hover:bg-surface-off-white-200 border-2 border-surface-off-white-500 text-typography-black-100 hover:text-typography-black-300 rounded-2xl transition-all cursor-pointer font-normal leading-5">
            <div className="flex gap-3 items-center">
              <div className="flex">
                <img
                  src="https://raw.githubusercontent.com/cosmos/chain-registry/master/_non-cosmos/ethereum/images/usdc.svg"
                  alt="usdc"
                  className="w-6 h-6 z-10"
                />
                <img
                  src="https://raw.githubusercontent.com/cosmos/chain-registry/master/_non-cosmos/ethereum/images/wsteth.svg"
                  alt="wseth"
                  className="w-6 h-6 ml-[-0.5rem]"
                />
              </div>
              <p>USDC - stETH</p>
            </div>
            <p>$192.08k</p>
            <p>$192.08k</p>
            <p>1.20%</p>
          </div>
          <div className="py-4 px-6 items-center gap-1 grid grid-cols-[1fr_80px_80px_80px] text-end bg-surface-rose-100 hover:bg-surface-off-white-200 border-2 border-surface-off-white-500 text-typography-black-100 hover:text-typography-black-300 rounded-2xl transition-all cursor-pointer font-normal leading-5">
            <div className="flex gap-3 items-center">
              <div className="flex">
                <img
                  src="https://raw.githubusercontent.com/cosmos/chain-registry/master/_non-cosmos/ethereum/images/usdc.svg"
                  alt="usdc"
                  className="w-6 h-6 z-10"
                />
                <img
                  src="https://raw.githubusercontent.com/cosmos/chain-registry/master/cosmoshub/images/atom.svg"
                  alt="cosmos"
                  className="w-6 h-6 ml-[-0.5rem]"
                />
              </div>
              <p>USDC - ATOM</p>
            </div>
            <p>$192.08k</p>
            <p>$192.08k</p>
            <p>1.20%</p>
          </div>
        </div>
      </div>
      <Button variant="light">Show all</Button>
    </GradientContainer>
  );
};
