"use client";

import { useAccount, useBalances, usePrices } from "@leftcurve/react";

import { Button } from "~/components";

import { formatAddress } from "@leftcurve/utils";
import { type VariantProps, tv } from "tailwind-variants";
import { twMerge } from "~/utils";

import { CardMarginBottom } from "./CardMarginBottom";
import { CardSafeBottom } from "./CardSafeBottom";
import { CardSpotBottom } from "./CardSpotBottom";

import { type Account, AccountType } from "@leftcurve/types";

export interface CardProps extends VariantProps<typeof cardVariants> {
  className?: string;
  avatarUrl: string;
  account: Account;
  onClick?: () => void;
  expanded?: boolean;
}

export const AccountCard: React.FC<CardProps> = ({
  className,
  onClick,
  account,
  avatarUrl,
  expanded,
}) => {
  const { calculateBalance } = usePrices();
  const { account: selectedAccount } = useAccount();
  const { isLoading, data: balances = {} } = useBalances({ address: account.address });
  const totalBalance = calculateBalance(balances, { format: true });
  const color = cardColors[account.type];
  const isActive = account.address === selectedAccount?.address;

  const { base, title, subtitle } = cardVariants();

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
      <div className={twMerge(base({ color: account.type, isActive }), className)}>
        <div className="flex items-start justify-between">
          <div className="flex gap-1 flex-col">
            <p
              className={twMerge(title({ color: account.type, isActive }))}
            >{`${account.type} account #${account.index}`}</p>
            <p className={twMerge(subtitle({ color: account.type, isActive }))}>
              {formatAddress(account.address)}
            </p>
          </div>
          <img
            src={avatarUrl}
            alt={`account ${account.type} - index ${account.index}`}
            className="h-16 w-16"
          />
        </div>
        <div>
          {account.type === AccountType.Spot ? (
            <CardSpotBottom isLoading={isLoading} totalBalance={totalBalance} />
          ) : null}
          {account.type === AccountType.Margin ? (
            <CardMarginBottom isLoading={isLoading} totalBalance={totalBalance} />
          ) : null}
          {account.type === AccountType.Safe ? (
            <CardSafeBottom
              isLoading={isLoading}
              totalBalance={totalBalance}
              members={(account as Account<typeof AccountType.Safe>).params.safe.members}
            />
          ) : null}
        </div>
      </div>
      {isActive ? (
        <Button variant="outline" color={color}>
          Manage
        </Button>
      ) : null}
    </div>
  );
};

const cardVariants = tv({
  slots: {
    base: "w-full min-h-[12rem] md:min-h-[10rem] rounded-2xl p-3 border-gray-200 border flex flex-col justify-between",
    title: "font-extrabold uppercase",
    subtitle: "text-sm font-normal",
  },
  variants: {
    color: {
      spot: {
        base: "bg-surface-rose-200",
        title: "text-typography-rose-600",
        subtitle: "text-typography-rose-500",
      },
      margin: {
        base: "bg-[#E0D6DA]",
        title: "text-typography-purple-400",
        subtitle: "text-typography-purple-300",
      },
      safe: {
        base: "bg-surface-yellow-300",
        title: "text-typography-yellow-400",
        subtitle: "text-typography-yellow-300",
      },
    },
    isActive: {
      true: {},
    },
  },
  compoundVariants: [
    {
      isActive: true,
      color: "spot",
      class: "border border-surface-rose-600/40",
    },
    {
      isActive: true,
      color: "margin",
      class: "border border-surface-purple-400/40",
    },
    {
      isActive: true,
      color: "safe",
      class: "border border-surface-yellow-400/40",
    },
  ],
});

const cardColors = {
  [AccountType.Spot]: "rose",
  [AccountType.Margin]: "purple",
  [AccountType.Safe]: "yellow",
} as const;
