"use client";

import { useAccount, useDisconnect } from "@leftcurve/react";
import type React from "react";
import { useRef, useState } from "react";
import { useClickAway } from "react-use";

import { truncateAddress } from "@leftcurve/utils";
import { twMerge } from "../../utils";

import { Button, CopyIcon, DangoButton, ProfileIcon, WalletIcon } from "../";

export const MenuConnections: React.FC = () => {
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [showMenu, setShowMenu] = useState(false);

  const { username, connector, account } = useAccount();
  const { disconnect } = useDisconnect();

  useClickAway(menuRef, (e) => {
    if (buttonRef.current?.contains(e.target as Node)) return;
    setShowMenu(false);
  });

  if (!connector || !account) return null;

  return (
    <>
      <DangoButton
        ref={buttonRef}
        onClick={() => setShowMenu(!showMenu)}
        color="gray"
        radius="lg"
        className="font-bold px-4 py-2 gap-2"
      >
        <ProfileIcon className="h-6 w-6" />
        <p>{username}</p>
      </DangoButton>

      <div
        ref={menuRef}
        className={twMerge(
          "transition-all bg-white backdrop-blur-3xl  min-w-[18rem] fixed top-[72px] right-4 rounded-3xl p-2",
          showMenu ? "h-fit border border-white" : "h-0 min-h-0 p-0 overflow-hidden border-none",
        )}
      >
        <div className="flex flex-col gap-3">
          <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
            CONNECTED WITH
          </p>
          {connector && (
            <div className="flex items-end justify-between py-4 px-4 rounded-2xl bg-green/80 text-white">
              <div className="flex gap-2">
                <div className="flex justify-center items-center h-12 w-12 rounded-2xl bg-sand-100/50">
                  <WalletIcon connectorId={connector.id} className="h-8 w-8" />
                </div>
                <div className="flex flex-col">
                  <p className="uppercase font-extrabold">{connector.name}</p>
                  <p className="text-md">{truncateAddress(account.address)}</p>
                </div>
              </div>
              <DangoButton
                color="none"
                size="sm"
                isIconOnly
                radius="md"
                className="px-2 bg-black/30 hover:bg-black/70"
              >
                <CopyIcon className="w-6 h-6" />
              </DangoButton>
            </div>
          )}
          <DangoButton color="purple" variant="bordered" size="sm">
            Manage Access
          </DangoButton>
          <DangoButton size="sm" onClick={() => disconnect({ connectorUId: connector.uid })}>
            Log out
          </DangoButton>
        </div>
      </div>
    </>
  );
};
