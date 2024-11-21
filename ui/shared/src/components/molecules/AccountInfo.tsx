"use client";

import { useAccount, useBalances, useConfig } from "@leftcurve/react";
import { formatNumber, formatUnits } from "@leftcurve/utils";
import { useAccountName } from "../../hooks";
import { Button } from "../atoms/Button";

interface Props {
  avatarUri?: string;
  triggerEdit?: () => void;
}

export const AccountInfo: React.FC<Props> = ({ avatarUri, triggerEdit }) => {
  const config = useConfig();
  const { account } = useAccount();
  const [accountName] = useAccountName();
  const { nativeCoin } = config.chains.find((chain) => chain.id === config.state.chainId)!;

  const { data: balances = {} } = useBalances({ address: account?.address });
  const nativeCoinBalance = formatUnits(balances[nativeCoin.denom] || 0, nativeCoin.decimals);

  if (!account) return null;

  return (
    <div className="dango-grid-square-mini-l flex flex-col gap-3 items-center justify-center text-sand-900">
      <div className="flex gap-2 text-sm w-full items-center justify-center font-extrabold text-typography-black-200">
        <p className="uppercase">{accountName}</p>
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
            <p>{formatNumber(nativeCoinBalance, { language: navigator.language! })}</p>
          </div>
        </div>
      </div>
      <Button variant="light" className="py-0" onClick={triggerEdit}>
        Rename
      </Button>
    </div>
  );
};
