import { useBalances, usePrices } from "@left-curve/store";

import { type Account, AccountType, type AccountTypes } from "@left-curve/dango/types";

import { AnimatePresence, motion } from "framer-motion";

import {
  Badge,
  BorrowBar,
  IconButton,
  IconChevronDownFill,
  IconClose,
  TextCopy,
  TruncateText,
  twMerge,
} from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

export const AccountCardOptions = {
  [AccountType.Spot]: {
    text: "Spot",
    badge: "blue",
    bgColor: "bg-account-card-red",
    img: "/images/characters/dog.svg",
    imgClassName: "opacity-60 right-[-2.9rem] bottom-[-4.3rem] scale-x-[-1] w-[14rem]",
  },
  [AccountType.Multi]: {
    text: "Multisig",
    badge: "green",
    bgColor: "bg-account-card-blue",
    img: "/images/characters/puppy.svg",
    imgClassName: "opacity-50 right-[-1rem] bottom-[-4.3rem] w-[15.4rem]",
  },
  [AccountType.Margin]: {
    text: "Margin",
    badge: "red",
    bgColor: "bg-account-card-green",
    img: "/images/characters/froggo.svg",
    imgClassName: "opacity-60 w-[15rem] bottom-[-5rem] right-[-0.5rem]",
  },
} as const;

type AccountCardProps = {
  account: Account;
  balance: string;
  balanceChange?: string;
  isSelectorActive?: boolean;
  onTriggerAction?: () => void;
};

const AccountCard: React.FC<AccountCardProps> = ({
  account,
  balance,
  balanceChange,
  onTriggerAction,
  isSelectorActive,
}) => {
  const { address, type } = account;
  const name = `${account?.type} #${account?.index}`;

  const { bgColor, badge, img, imgClassName, text } = AccountCardOptions[type];

  return (
    <div
      className={twMerge(
        "shadow-account-card w-full max-w-[22.5rem] md:max-w-[20.5rem] lg:min-w-[20.5rem] h-[10rem] relative overflow-hidden rounded-xl flex flex-col justify-between p-4 text-secondary-700",
        bgColor,
      )}
    >
      <img
        src={img}
        alt="account-card-dog"
        className={twMerge("absolute right-0 bottom-0 select-none drag-none", imgClassName)}
      />
      <AnimatePresence mode="wait">
        {onTriggerAction ? (
          <IconButton
            className="absolute top-4 right-4 z-30"
            size="sm"
            variant="secondary"
            onClick={() => onTriggerAction()}
          >
            <motion.span
              key={isSelectorActive ? "selector" : "assets"}
              initial={{ scale: 0.5 }}
              animate={{ scale: 1 }}
              transition={{ duration: 0.2 }}
              exit={{ scale: 0.5 }}
            >
              {isSelectorActive ? (
                <IconClose className="w-5 h-5" />
              ) : (
                <IconChevronDownFill className="w-4 h-4" />
              )}
            </motion.span>
          </IconButton>
        ) : null}
      </AnimatePresence>
      <div className="flex flex-col relative z-10">
        <div className="flex gap-1 items-center">
          <p className="exposure-m-italic capitalize">{name}</p>
          <Badge text={text} color={badge} className="h-fit capitalize" size="s" />
        </div>
        <div className="flex gap-1 items-center">
          <TruncateText
            text={address}
            className="diatype-xs-medium text-tertiary-500"
            start={4}
            end={4}
          />
          <TextCopy copyText={address} className="w-4 h-4 cursor-pointer text-tertiary-500" />
        </div>
      </div>
      {type === AccountType.Margin ? (
        <BorrowBar borrow={0} borrowed={0} total={0} />
      ) : (
        <div className="flex gap-2 items-center relative z-10">
          <p className="h4-regular">{balance}</p>
          <p className="text-sm font-bold text-status-success">{balanceChange}</p>
        </div>
      )}
    </div>
  );
};

type AccountCardPreviewProps = {
  account: Account;
  onAccountSelect: (account: Account) => void;
};

const Preview: React.FC<AccountCardPreviewProps> = ({ account, onAccountSelect }) => {
  const { address } = account;

  const type = account?.type as AccountTypes;
  const name = `${type} #${account?.index}`;

  const { bgColor, badge, text } = AccountCardOptions[type];

  const { data: balances = {} } = useBalances({ address });
  const { calculateBalance } = usePrices();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const totalBalance = calculateBalance(balances, {
    format: true,
    formatOptions: { ...formatNumberOptions, currency: "usd" },
  });

  return (
    <div
      className={twMerge(
        "shadow-account-card w-full max-w-[22.5rem] md:max-w-[20.5rem] lg:min-w-[20.5rem] mb-[-6.2rem] flex-shrink-0 h-[10rem] relative overflow-hidden rounded-xl flex flex-col justify-between p-4 cursor-pointer text-secondary-700",
        bgColor,
      )}
      onClick={() => onAccountSelect(account)}
    >
      <div className="flex items-start justify-between relative z-10">
        <div className="flex flex-col">
          <div className="flex gap-1 items-center">
            <p className="exposure-m-italic capitalize text-tertiary-500">{name}</p>
          </div>
          <div className="flex gap-1 items-center">
            <TruncateText
              text={address}
              className="diatype-xs-medium text-tertiary-500"
              start={4}
              end={4}
            />
            <TextCopy copyText={address} className="w-4 h-4 cursor-pointer text-tertiary-500" />
          </div>
        </div>
        <div className="flex flex-col gap-1 items-end">
          <p className="diatype-m-bold text-tertiary-500">{totalBalance}</p>
          <Badge text={text} color={badge} className="h-fit capitalize" size="s" />
        </div>
      </div>
    </div>
  );
};

const ExportContainer = Object.assign(AccountCard, {
  Preview,
});

export { ExportContainer as AccountCard };
