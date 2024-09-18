"use client";

import { useAccount } from "~/hooks";

import { Button } from "~/components";

import { formatAddress } from "@leftcurve/utils";
import { type VariantProps, tv } from "tailwind-variants";
import { twMerge } from "~/utils";

import { type Account, AccountType } from "@leftcurve/types";

export interface CardProps extends VariantProps<typeof cardVariants> {
  className?: string;
  account: Account;
  onClick?: () => void;
  expanded?: boolean;
}

export const AccountCard: React.FC<CardProps> = ({ className, account, onClick, expanded }) => {
  const { account: selectedAccount } = useAccount();
  const balance = "$0.00"; // TODO: Get balance
  const color = cardColors[account.type];
  return (
    <div
      className={twMerge(
        "flex flex-col gap-2 transition-all",
        expanded
          ? "first:mt-0  mt-0"
          : "first:mt-0 first:mb-[9rem] first:md:mb-[6.5rem] mt-[-9rem] md:mt-[-6.5rem]",
      )}
      onClick={onClick}
    >
      <div className={twMerge(cardVariants({ color, className }))}>
        <div className="flex items-start justify-between">
          <div className="flex gap-1 flex-col">
            <p className="font-extrabold uppercase">{`${account.type} account #${account.index}`}</p>
            <p className="text-xs">{formatAddress(account.address)}</p>
          </div>
          <img
            src="https://static.thenounproject.com/png/2616533-200.png"
            alt={`account ${account.type} - index ${account.index}`}
            className="h-16 w-16"
          />
        </div>
        <div>
          {
            account.type.includes(AccountType.Safe) && null // TODO: Avatar using identicon
          }
          <div className="flex items-center justify-between">
            <p className="uppercase text-sm">Balance:</p>
            <p className="text-lg font">{balance}</p>
          </div>
        </div>
      </div>
      {account.address === selectedAccount?.address ? <Button color={color}>Manage</Button> : null}
    </div>
  );
};

const cardVariants = tv({
  base: "w-full min-h-[12rem] md:min-h-[10rem] rounded-2xl p-3 border-gray-200 border flex flex-col justify-between",
  variants: {
    color: {
      default: "bg-white text-gray-900 ",
      sand: "bg-sand text-sand-900 border-sand-900",
      green: "bg-green text-green-900 border-green-900",
      purple: "bg-purple-300 text-purple-900 border-purple-900",
    },
  },
  defaultVariants: {
    color: "default",
  },
});

const cardColors = {
  [AccountType.Spot]: "purple",
  [AccountType.Margin]: "sand",
  [AccountType.Safe]: "green",
} as const;
