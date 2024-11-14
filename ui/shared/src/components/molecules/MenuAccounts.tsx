"use client";

import { useAccount } from "@leftcurve/react";
import { useMemo, useRef, useState } from "react";
import { useClickAway } from "react-use";

import { twMerge, useDOMRef } from "../../utils";

import { AccountCard, Button } from "../";
import { CloseIcon, CollapseIcon, ExpandedIcon, PlusIcon } from "../";

import { type Account, AccountType } from "@leftcurve/types";
import { capitalize } from "@leftcurve/utils";
import { useAccountName } from "../../hooks";

interface Props {
  createAction?: () => void;
  manageAction?: (account: Account) => void;
  images: {
    [AccountType.Spot]: string;
    [AccountType.Margin]: string;
    [AccountType.Safe]: string;
  };
}

export const MenuAccounts: React.FC<Props> = ({ images, createAction, manageAction }) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useDOMRef<HTMLButtonElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [showMenu, setShowMenu] = useState(false);
  const { account: selectedAccount, accounts, changeAccount } = useAccount();
  const [accountName] = useAccountName();

  useClickAway(menuRef, (e) => {
    if (buttonRef.current?.contains(e.target as Node)) return;
    setShowMenu(false);
  });

  const handleAction = (account: Account) => {
    manageAction?.(account);
    setShowMenu(false);
  };

  const sortedAccounts = useMemo(() => {
    return [...(accounts ? accounts : [])]?.sort((a, b) => {
      if (selectedAccount?.index === a.index) return -1;
      return a.index - b.index;
    });
  }, [selectedAccount, accounts]);

  if (!selectedAccount) return null;

  return (
    <>
      <Button
        ref={buttonRef}
        onClick={() => setShowMenu(!showMenu)}
        color="gray"
        radius="lg"
        className="font-bold px-4 py-2"
      >
        {capitalize(accountName)}
      </Button>
      <div
        ref={menuRef}
        className={twMerge(
          "transition-all bg-white/50 backdrop-blur-3xl w-full md:w-[19.5rem] fixed top-0 md:top-[72px] md:rounded-3xl p-4 md:p-2 md:py-4 flex flex-col gap-4 h-[100vh] md:max-h-[calc(100vh-78px)] z-50",
          showMenu ? "right-0 md:right-4" : "right-[-100vh]",
        )}
      >
        <div
          className={twMerge("flex items-center ", expanded ? "justify-center" : "justify-between")}
        >
          <p className="text-2xl font-extrabold font-diatype-rounded mx-2 tracking-widest flex-1">
            Accounts
          </p>
          <div className="flex gap-2">
            <Button
              isIconOnly
              radius="lg"
              color="green"
              onClick={() => [createAction?.(), setShowMenu(false)]}
            >
              <PlusIcon className="h-6 w-6" />
            </Button>
            <Button color="purple" radius="lg" isIconOnly onClick={() => setExpanded(!expanded)}>
              {expanded ? (
                <CollapseIcon className="h-6 w-6" />
              ) : (
                <ExpandedIcon className="h-6 w-6" />
              )}
            </Button>
            <Button onClick={() => setShowMenu(false)} color="rose" radius="lg" isIconOnly>
              <CloseIcon className="h-6 w-6" />
            </Button>
          </div>
        </div>
        <div
          className={twMerge(
            "flex flex-col gap-4 relative flex-1 scrollbar-none",
            expanded ? "overflow-scroll" : "overflow-hidden",
          )}
        >
          {sortedAccounts.map((account) => {
            return (
              <AccountCard
                avatarUrl={images[account.type]}
                key={account.index}
                account={account}
                onClick={() => [changeAccount?.(account), setExpanded(false)]}
                manageAction={handleAction}
                expanded={expanded}
              />
            );
          })}

          <div
            className={twMerge(
              "absolute bottom-0 left-0 w-full h-[2rem] bg-gradient-to-b from-transparent to-white/50  z-[60]",
              expanded ? "scale-0" : "scale-100",
            )}
          />
          <div
            className={twMerge(
              "absolute top-[16rem] left-0 w-full h-[calc(100%-16rem)] md:top-[14rem] md:h-[calc(100%-14rem)] bg-transparent",
              expanded ? "scale-0" : "scale-100",
            )}
            onClick={() => setExpanded(true)}
          />
        </div>
      </div>
    </>
  );
};
