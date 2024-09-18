"use client";

import { useMemo, useRef, useState } from "react";
import { useClickAway } from "react-use";
import { useAccount } from "~/hooks";

import { capitalize } from "@leftcurve/utils";
import { twMerge } from "~/utils";

import { AccountCard, Button } from "~/components";
import { CloseIcon, CollapseIcon, ExpandedIcon, PlusIcon } from "~/components";

export const MenuAccounts: React.FC = () => {
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [showMenu, setShowMenu] = useState(false);
  const { account: selectedAccount, accounts, changeAccount } = useAccount();

  useClickAway(menuRef, (e) => {
    if (buttonRef.current?.contains(e.target as Node)) return;
    setShowMenu(false);
  });

  const sortedAccounts = useMemo(() => {
    return [...(accounts ? accounts : [])]?.sort((a, b) => {
      if (selectedAccount?.index === a.index) return -1;
      return a.index - b.index;
    });
  }, [selectedAccount, accounts]);

  if (!selectedAccount) return null;

  return (
    <>
      <Button ref={buttonRef} onClick={() => setShowMenu(!showMenu)}>
        {capitalize(selectedAccount.type)} Account #{selectedAccount.index}
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
            <Button className="h-10 w-10 px-2 rounded-xl" color="green">
              <PlusIcon className="h-6 w-6" />
            </Button>
            <Button
              className="h-10 w-10 px-2 rounded-xl"
              onClick={() => setExpanded(!expanded)}
              color="sand"
            >
              {expanded ? (
                <CollapseIcon className="h-6 w-6" />
              ) : (
                <ExpandedIcon className="h-6 w-6" />
              )}
            </Button>
            <Button
              className="h-10 w-10 px-2 rounded-xl"
              onClick={() => setShowMenu(false)}
              color="danger"
            >
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
                key={account.index}
                account={account}
                onClick={() => [changeAccount?.(account), setExpanded(false)]}
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
