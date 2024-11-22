"use client";

import { useAccount, useDisconnect } from "@leftcurve/react";
import type React from "react";
import { useRef, useState } from "react";
import { useClickAway } from "react-use";

import { truncateAddress } from "@leftcurve/utils";
import { twMerge } from "../../utils";

import { Button, CopyCheckIcon, CopyIcon, ProfileIcon, WalletIcon } from "../";

export const MenuConnections: React.FC = () => {
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [showMenu, setShowMenu] = useState(false);
  const [copyIcon, setCopyIcon] = useState(<CopyIcon className="w-6 h-6" />);

  const { username, connector, account } = useAccount();
  const { disconnect } = useDisconnect();

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
        radius="lg"
        className="font-bold px-4 py-2 gap-2"
      >
        <ProfileIcon className="h-6 w-6" />
        <p className="truncate">{username}</p>
      </Button>

      <div
        ref={menuRef}
        className={twMerge(
          "transition-all bg-white backdrop-blur-3xl  min-w-[18rem] fixed top-[72px] right-4 rounded-3xl p-2 z-[90]",
          showMenu ? "h-fit border border-white" : "h-0 min-h-0 p-0 overflow-hidden border-none",
        )}
      >
        <div className="flex flex-col gap-3">
          <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
            CONNECTED WITH
          </p>
          {connector && (
            <div className="flex items-end justify-between py-4 px-4 rounded-2xl bg-brand-green/80 text-white">
              <div className="flex gap-2">
                <div className="flex justify-center items-center h-12 w-12 rounded-2xl bg-sand-100/50">
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
                className="px-2 bg-black/30 hover:bg-black/70"
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
};
