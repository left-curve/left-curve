import { twMerge, useMediaQuery } from "@left-curve/applets-kit";
import { useAccount, useBalances, usePrices } from "@left-curve/store-react";
import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import { useApp } from "~/hooks/useApp";
import { AccountCard } from "./AccountCard";
import { AssetsPreview } from "./AssetsPreview";

const variants = {
  enter: (direction: number) => ({
    x: direction > 0 ? 300 : -300,
    opacity: 0,
  }),
  center: {
    x: 0,
    opacity: 1,
  },
  exit: (direction: number) => ({
    x: direction > 0 ? -300 : 300,
    opacity: 0,
  }),
};

interface Props {
  cardVisible: number;
  setCardVisible: (value: number) => void;
}

export const SwippeableAccountCard: React.FC<Props> = ({ cardVisible, setCardVisible }) => {
  const { account, isConnected } = useAccount();
  const { setSidebarVisibility } = useApp();
  const [direction, setDirection] = useState(0);
  const { data: balances = {} } = useBalances({ address: account?.address });
  const { formatNumberOptions } = useApp();
  const { calculateBalance } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const totalBalance = calculateBalance(balances, {
    format: true,
    formatOptions: {
      ...formatNumberOptions,
      currency: "USD",
    },
  });
  const { isLg } = useMediaQuery();

  if (isLg) return <AccountCard account={account!} balance={totalBalance} />;

  return (
    <AnimatePresence initial={false} mode="wait" custom={direction}>
      <motion.div
        key={cardVisible}
        custom={direction}
        variants={variants}
        initial="enter"
        animate="center"
        exit="exit"
        transition={{ duration: 0.3 }}
        className="w-full lg:w-fit items-center flex justify-center"
        drag="x"
        dragConstraints={{ left: 0, right: 0 }}
        onDragEnd={(event, info) => {
          if (info.offset.x > 50) {
            setCardVisible(0);
            setDirection(-1);
          } else if (info.offset.x < -50) {
            setCardVisible(1);
            setDirection(1);
          }
        }}
      >
        {cardVisible === 0 ? (
          <AccountCard account={account!} balance={totalBalance} />
        ) : (
          <div className="flex lg:hidden w-full max-w-[20.5rem] h-[9.75rem]">
            <AssetsPreview
              balances={balances}
              showAllAssets={isConnected ? () => setSidebarVisibility(true) : undefined}
            />
          </div>
        )}
      </motion.div>
    </AnimatePresence>
  );
};

export const DotsIndicator: React.FC<Props> = ({ cardVisible, setCardVisible }) => {
  return (
    <div className="dots flex w-full items-center justify-center gap-3 lg:hidden">
      <div
        onClick={() => setCardVisible(0)}
        className={twMerge(
          "w-[10px] h-[10px] rounded-full cursor-pointer transition-colors",
          cardVisible === 0 ? "bg-rice-300" : "bg-rice-200",
        )}
      />
      <div
        onClick={() => setCardVisible(1)}
        className={twMerge(
          "w-[10px] h-[10px] rounded-full cursor-pointer transition-colors",
          cardVisible === 1 ? "bg-rice-300" : "bg-rice-200",
        )}
      />
    </div>
  );
};
