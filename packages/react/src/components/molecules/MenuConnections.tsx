import type React from "react";
import { useRef, useState } from "react";
import { useClickAway } from "react-use";
import { useAccount, useDisconnect } from "~/hooks";

import { formatAddress } from "@leftcurve/utils";
import { twMerge } from "~/utils";

import { Button, WalletIcon } from "~/components";
import { CopyIcon, ProfileIcon } from "~/components";

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
      <Button
        ref={buttonRef}
        className="items-center flex gap-2"
        onClick={() => setShowMenu(!showMenu)}
      >
        <ProfileIcon className="h-6 w-6" />
        <p>{username}</p>
      </Button>

      <div
        ref={menuRef}
        className={twMerge(
          "transition-all bg-white backdrop-blur-3xl  min-w-[18rem] fixed top-[72px] right-4 rounded-3xl p-2",
          showMenu ? "h-fit border border-white" : "h-0 min-h-0 p-0 overflow-hidden border-none",
        )}
      >
        <div className="flex flex-col gap-3">
          <p className="text-sm font-bold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
            CONNECTED WITH
          </p>
          {connector && (
            <div className="flex items-end justify-between py-4 px-4 rounded-2xl bg-green text-white">
              <div className="flex gap-2">
                <div className="flex justify-center items-center h-12 w-12 rounded-2xl bg-sand-100/50">
                  <WalletIcon connectorId={connector.id} className="h-8 w-8" />
                </div>
                <div className="flex flex-col">
                  <p className="uppercase font-extrabold">{connector.name}</p>
                  <p className="text-md">{formatAddress(account.address)}</p>
                </div>
              </div>
              <Button color="green" variant="dark" size="icon" className="p-1 h-8 w-8 rounded-lg">
                <CopyIcon />
              </Button>
            </div>
          )}
          <Button color="sand" variant="flat">
            Manage Access
          </Button>
          <Button
            color="danger"
            variant="solid"
            onClick={() => disconnect({ connectorUId: connector.uid })}
          >
            Log out
          </Button>
        </div>
      </div>
    </>
  );
};
