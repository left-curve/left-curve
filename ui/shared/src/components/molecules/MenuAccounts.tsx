"use client";

import { useAccount } from "@left-curve/react";
import { forwardRef, useImperativeHandle, useMemo, useRef, useState } from "react";
import { useClickAway } from "react-use";

import { twMerge } from "../../utils";

import { AccountCard, Button } from "../";

import { type Account, AccountType } from "@left-curve/types";
import { capitalize } from "@left-curve/utils";
import { useAccountName } from "../../hooks";
import type { VisibleRef } from "../../types";
import { CrossIcon } from "../icons/Cross";

interface Props {
  manageAction?: (account: Account) => void;
  images: {
    [AccountType.Spot]: string;
    [AccountType.Margin]: string;
    [AccountType.Safe]: string;
  };
}

export const MenuAccounts = forwardRef<VisibleRef, Props>(({ images, manageAction }, ref) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [showMenu, setShowMenu] = useState(false);

  const { account: selectedAccount, accounts, changeAccount } = useAccount();
  const [accountName] = useAccountName();
  const [expanded, setExpanded] = useState(false);

  useImperativeHandle(ref, () => ({
    isVisible: showMenu,
    changeVisibility: (v) => setShowMenu(v),
  }));

  useClickAway(menuRef, (e) => {
    if (buttonRef.current?.contains(e.target as Node)) return;
    setShowMenu(false);
    setExpanded(false);
  });

  const sortedAccounts = useMemo(() => {
    return [...(accounts ? accounts : [])]?.sort((a, b) => a.index - b.index);
  }, [accounts]);

  if (!selectedAccount) return null;

  return (
    <>
      <Button
        ref={buttonRef}
        onClick={() => setShowMenu(!showMenu)}
        color="gray"
        className="font-bold px-4 py-2 min-w-32 hidden lg:flex rounded-[20px] overflow-hidden"
      >
        <span className="font-diatype-rounded truncate">{capitalize(accountName)}</span>
      </Button>

      <div
        ref={menuRef}
        className={twMerge(
          "transition-all bg-white/50 backdrop-blur-3xl w-full md:w-[19.5rem] fixed top-0 md:top-[72px] md:rounded-3xl p-4 md:p-2 md:py-4 flex flex-col gap-4 h-[100vh] md:max-h-[calc(100vh-78px)] z-50 duration-300 delay-100",
          showMenu ? "right-0 md:right-4" : "right-[-100vh]",
        )}
      >
        <div
          className={twMerge("flex items-center ", expanded ? "justify-center" : "justify-between")}
        >
          <p className="text-2xl font-extrabold font-diatype-rounded mx-2 tracking-widest flex-1 text-typography-black-200">
            Accounts
          </p>
          <p
            className="p-2 bg-surface-green-300 rounded-xl text-typography-black-300 lg:hidden"
            onClick={() => setShowMenu(false)}
          >
            <CrossIcon className="w-4 h-4" />
          </p>
        </div>

        <div className="relative flex-1 overflow-hidden flex flex-col gap-4">
          <div className="flex flex-col w-full gap-2">
            <AccountCard avatarUrl={images[selectedAccount.type]} account={selectedAccount} />
            <Button
              variant="bordered"
              color="purple"
              size="sm"
              onClick={() => manageAction?.(selectedAccount)}
            >
              Manage
            </Button>
          </div>
          <div
            className={twMerge(
              "flex flex-col gap-4 relative flex-1 scrollbar-none",
              expanded ? "overflow-scroll" : "overflow-hidden cursor-pointer",
            )}
          >
            {sortedAccounts.map((account) => {
              if (account.index === selectedAccount.index) return null;
              return (
                <AccountCard
                  avatarUrl={images[account.type]}
                  key={account.index}
                  account={account}
                  onClick={() => [changeAccount?.(account), setExpanded(false)]}
                  expanded={expanded}
                />
              );
            })}
          </div>

          <div
            className={twMerge(
              "absolute bottom-0 left-0 w-full h-[2rem] bg-gradient-to-b from-transparent to-white/50 z-[60]",
              !expanded ? "scale-0" : "scale-100",
            )}
          />
          <div
            className={twMerge(
              "absolute top-[16rem] left-0 w-full h-[calc(100%-16rem)] md:top-[14rem] md:h-[calc(100%-14rem)] bg-transparent",
              expanded ? "scale-0" : "scale-100",
              { "scale-0": Boolean(accounts?.length && accounts?.length <= 2) },
            )}
            onClick={() => setExpanded(true)}
          />
        </div>
      </div>
    </>
  );
});
