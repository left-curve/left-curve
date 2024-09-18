"use client";

import { ConnectorStatus } from "@leftcurve/types";
import { ConnectButton, MenuAccounts, MenuConnections, MenuNotifications } from "~/components";
import { useAccount } from "~/hooks";

export const ExampleHeader: React.FC = () => {
  const { status } = useAccount();
  return (
    <header className="flex h-16 w-full items-center justify-between px-4 md:px-6 bg-white">
      <div className="flex items-center gap-2">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          className="w-6 h-6 text-primary-500"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="m8 3 4 8 5-5 5 15H2L8 3z" />
        </svg>
        <span className="text-lg font-semibold">Example App</span>
      </div>
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
  );
};
