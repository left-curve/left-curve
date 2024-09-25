"use client";

import { useAccount } from "@leftcurve/react";

import { CommandBar } from "~/components";
import { ConnectButton } from "./ConnectButton";
import { MenuAccounts } from "./MenuAccounts";
import { MenuConnections } from "./MenuConnections";
import { MenuNotifications } from "./MenuNotifications";

import { ConnectorStatus } from "@leftcurve/types";

export const Header: React.FC = () => {
  const { status } = useAccount();

  return (
    <>
      <header className="fixed top-0 left-0 gap-4 z-50 flex flex-wrap md:flex-nowrap items-center justify-between w-full p-4 xl:grid xl:grid-cols-4">
        <div className="h-10 w-10 rounded-full bg-gray-200 order-1" />
        <CommandBar applets={[]} />
        <div className="flex gap-2 items-center justify-end order-2 md:order-3">
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
