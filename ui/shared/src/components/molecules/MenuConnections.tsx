"use client";

import { useAccount, useDisconnect } from "@left-curve/react";

import { forwardRef, useImperativeHandle, useRef, useState } from "react";
import { useClickAway } from "react-use";

import { truncateAddress } from "@left-curve/utils";
import { twMerge } from "../../utils";

import { Button, CopyCheckIcon, CopyIcon, ProfileIcon, WalletIcon } from "../";
import type { VisibleRef } from "../../types";
import { CrossIcon } from "../icons/Cross";

export const MenuConnections = forwardRef<VisibleRef>((props, ref) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [showMenu, setShowMenu] = useState(false);
  const [copyIcon, setCopyIcon] = useState(<CopyIcon className="w-6 h-6" />);

  const { connector, account } = useAccount();
  const { disconnect } = useDisconnect();

  useImperativeHandle(ref, () => ({
    isVisible: showMenu,
    changeVisibility: (v) => setShowMenu(v),
  }));

  useClickAway(menuRef, (e) => {
    if (buttonRef.current?.contains(e.target as Node)) return;
    setShowMenu(false);
  });

  const copyAction = () => {
    if (!account) return;
    navigator.clipboard.writeText(account.address);
    setCopyIcon(<CopyCheckIcon className="w-6 h-6" />);
    setTimeout(() => setCopyIcon(<CopyIcon className="w-6 h-6" />), 1000);
  };

  if (!connector || !account) return null;

  return (
    <>
      <Button
        ref={buttonRef}
        onClick={() => setShowMenu(!showMenu)}
        color="gray"
        className="font-bold px-4 py-2 gap-2 hidden lg:flex rounded-[20px]"
      >
        <ProfileIcon className="h-6 w-6 text-surface-green-100" />
        <p className="truncate">{account?.username}</p>
      </Button>

      <div
        ref={menuRef}
        className={twMerge(
          "transition-all bg-white/50 backdrop-blur-3xl w-full lg:w-[19.5rem] fixed top-0 lg:top-[72px] lg:rounded-3xl p-4 lg:p-2 lg:py-4 flex flex-col gap-4 h-[100vh] lg:h-fit lg:max-h-[calc(100vh-78px)] z-50 duration-300 delay-100",
          showMenu ? "right-0 lg:right-4" : "right-[-100vh]",
        )}
      >
        <div className="flex flex-col gap-3">
          <div className="flex justify-between items-center">
            <p className="text-xl lg:text-sm font-extrabold text-typography-black-200 font-diatype-rounded mx-2 tracking-widest">
              CONNECTED WITH
            </p>
            <p
              className="p-2 bg-surface-green-300 rounded-xl text-typography-black-300 lg:hidden"
              onClick={() => setShowMenu(false)}
            >
              <CrossIcon className="w-4 h-4" />
            </p>
          </div>
          {connector && (
            <div className="flex items-end justify-between py-4 px-4 rounded-2xl bg-surface-green-300 text-typography-black-200">
              <div className="flex gap-2">
                <div className="flex justify-center items-center h-12 w-12 rounded-2xl bg-surface-rose-300/50">
                  <WalletIcon connectorId={connector.id} className="h-8 w-8" />
                </div>
                <div className="flex flex-col">
                  <p className="uppercase font-extrabold">{connector.name}</p>
                  <p className="text-md">{truncateAddress(account.address)}</p>
                </div>
              </div>
              <Button
                color="none"
                size="sm"
                isIconOnly
                radius="md"
                className="px-2 bg-typography-black-200/30 text-white"
                onClick={copyAction}
              >
                {copyIcon}
              </Button>
            </div>
          )}
          <Button size="sm" onClick={() => disconnect({ connectorUId: connector.uid })}>
            Log out
          </Button>
        </div>
      </div>
    </>
  );
});
