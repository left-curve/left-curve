"use client";

import { useAccount, useBalances, useConfig } from "@leftcurve/react";
import { formatNumber } from "@leftcurve/utils";
import { Button } from "../atoms/Button";

import type { Language } from "@leftcurve/types";

interface Props {
  avatarUri?: string;
}

export const AccountInfo: React.FC<Props> = ({ avatarUri }) => {
  const config = useConfig();
  const { account } = useAccount();
  const { nativeCoin } = config.chains.find((chain) => chain.id === config.state.chainId)!;

  const { data: balances = {} } = useBalances({ address: account?.address });
  const nativeCoinBalance = balances[nativeCoin.denom] || 0;
  const language = navigator.language as Language;

  if (!account) return null;

  return (
    <div className="bg-gradient-to-br from-sand-100/70 to-white/10 backdrop-blur-sm  rounded-3xl flex flex-col gap-3 items-center justify-between text-sand-900 p-4 sm:min-w-[18rem] sm:w-fit w-full  min-h-[18rem]">
      <div className="flex gap-2 text-sm w-full items-center justify-center font-extrabold text-typography-black-200">
        <p className="uppercase">{account.username}</p>
        <p className="uppercase">
          {account.type} #{account.index}
        </p>
      </div>
      <div className="rounded-full bg-surface-rose-200 p-4">
        {avatarUri ? (
          <img
            src={avatarUri}
            className="rounded-full h-[4.5rem] w-[4.5rem]"
            alt="account-type-avatar"
          />
        ) : (
          <div className="rounded-full h-[4.5rem] w-[4.5rem] bg-gray-200" />
        )}
      </div>
      <div className="flex flex-col gap-2 w-full">
        <div className="flex items-center justify-between gap-2">
          <p className="uppercase text-sm font-bold tracking-[0.175rem]">BALANCE</p>
          <div className="flex gap-1 font-extrabold text-lg">
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
