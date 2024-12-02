"use client";

import { useBalances, usePrices } from "@left-curve/react";

import { motion } from "framer-motion";

import { capitalize, truncateAddress } from "@left-curve/utils";
import { type VariantProps, tv } from "tailwind-variants";
import { twMerge } from "../../../utils";

import { CardMarginBottom } from "./CardMarginBottom";
import { CardSafeBottom } from "./CardSafeBottom";
import { CardSpotBottom } from "./CardSpotBottom";

import { type Account, AccountType } from "@left-curve/types";
import { useAccountName, useMediaQuery } from "../../../hooks";

export interface CardProps extends VariantProps<typeof cardVariants> {
  className?: string;
  avatarUrl: string;
  account: Account;
  isActive: boolean;
  index: number;
  expandedIndex: number;
  onChangeExpand: (address?: string) => void;
  onAccountSelection: () => void;
}

export const AccountCard: React.FC<CardProps> = ({
  className,
  onAccountSelection,
  account,
  avatarUrl,
  isActive,
  onChangeExpand,
  index,
  expandedIndex,
}) => {
  const { calculateBalance } = usePrices();
  const [accountName] = useAccountName({ account });
  const { isLoading, data: balances = {} } = useBalances({ address: account.address });
  const totalBalance = calculateBalance(balances, { format: true });
  const isMd = useMediaQuery("md");

  const isExpanded = expandedIndex === index;

  const { base, title, subtitle } = cardVariants();

  const handleInteraction = () => {
    if (isExpanded) {
      onAccountSelection();
    } else {
      onChangeExpand(account.address);
    }
  };

  return (
    <>
      <motion.div
        className={twMerge(
          "flex flex-col gap-2 transition-all cursor-pointer absolute w-full",
          { "z-[222] mb-[9rem] last:mb-0 md:mb-[6rem]": isExpanded },
          { " z-[2222]": isActive },
        )}
        animate={{
          top: (() => {
            const expandSize = isMd ? 110 : 125;
            const cardSize = isMd ? 160 : 180;
            const cardInvisibleArea = 96;

            const startPosition = cardSize - 52 + index * (cardSize - cardInvisibleArea);
            if (index === 0) return 0;
            if (expandedIndex > 0 && index > expandedIndex) return startPosition + expandSize;
            return startPosition;
          })(),
        }}
        transition={{
          ease: "easeIn",
          duration: expandedIndex > 0 ? 0.2 : 0.3,
        }}
        onMouseEnter={() => isMd && onChangeExpand(account.address)}
        onMouseLeave={() => isMd && onChangeExpand()}
        onClick={handleInteraction}
      >
        <div className={twMerge(base({ color: account.type, isActive }), className)}>
          <div className="flex items-start justify-between">
            <div className="flex gap-1 flex-col">
              <p className={twMerge(title({ color: account.type, isActive }))}>
                {capitalize(accountName)}
              </p>
              <p className={twMerge(subtitle({ color: account.type, isActive }))}>
                {truncateAddress(account.address)}
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
      </motion.div>
    </>
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
