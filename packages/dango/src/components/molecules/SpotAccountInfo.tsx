"use client";

import { useAccount, useBalances, useConfig } from "@leftcurve/react";
import { formatAddress, formatNumber } from "@leftcurve/utils";
import { Button } from "~/components";

import type { Language } from "@leftcurve/types";

export const SpotAccountInfo: React.FC = () => {
  const config = useConfig();
  const { account } = useAccount();
  const { nativeCoin } = config.chains.find((chain) => chain.id === config.state.chainId)!;

  const { data: balances = {} } = useBalances({ address: account!.address });
  const nativeCoinBalance = balances[nativeCoin.denom] || 0;
  const language = navigator.language as Language;

  if (!account) return null;

  return (
    <div className="bg-gradient-to-br from-sand-100/70 to-white/10 backdrop-blur-sm  rounded-3xl flex flex-col gap-3 items-center justify-center text-sand-900 p-4 sm:min-w-[18rem] sm:w-fit w-full  min-h-[18rem]">
      <div className="flex gap-2 text-sm w-full items-center justify-center">
        <p className="uppercase">{account.username}</p>
        <p>{formatAddress(account.address)}</p>
      </div>
      <div className="rounded-full bg-white p-2">
        <img src="https://via.placeholder.com/150" className="rounded-full h-16 w-16" alt="test" />
      </div>
      <div className="flex flex-col gap-2 w-full">
        <div className="flex items-center justify-between gap-2 ">
          <p className="uppercase text-sm font-light">FUEL</p>
          <div className="flex gap-1 font-bold">
            <p className="text-sand-800/50">{nativeCoin.symbol}</p>
            <p>{formatNumber(balances[nativeCoin.denom] || 0, { language })}</p>
          </div>
        </div>
        <div className="flex items-center justify-between gap-2 ">
          <p className="uppercase text-sm font-light">BALANCE</p>
          <div className="flex gap-1 font-bold">
            <p className="text-sand-800/50">$</p>
            <p>{formatNumber(nativeCoinBalance, { language })}</p>
          </div>
        </div>
      </div>
      <Button color="sand" variant="light" className="italic">
        Edit
      </Button>
    </div>
  );
};
