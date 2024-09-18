"use client";

import { useAccount } from "~/hooks";

import { ConnectButton } from "./ConnectButton";
import { MenuAccounts } from "./MenuAccounts";
import { MenuConnections } from "./MenuConnections";
import { MenuNotifications } from "./MenuNotifications";

import { ConnectorStatus } from "@leftcurve/types";

export const Header: React.FC = () => {
  const { status } = useAccount();

  return (
    <>
      <header className="fixed top-0 left-0 flex gap-4 z-50 items-center justify-between w-full p-4">
        <div className="h-10 w-10 rounded-full bg-gray-200" />
        <div className="flex gap-2 items-center justify-between">
          {ConnectorStatus.Connected === status ? (
            <>
              <MenuNotifications />
              <MenuAccounts />
              <MenuConnections />
            </>
          ) : (
            <ConnectButton />
          )}
        </div>
      </header>
    </>
  );
};
