"use client";

import { useAccount } from "~/hooks";

export const ExampleAccountList: React.FC = () => {
  const { isConnected, accounts } = useAccount();
  return (
    <div className="flex flex-1 justify-center items-center">
      {accounts?.length ? (
        <div>
          <div className="flex flex-col items-center justify-center h-full">
            <p className="text-2xl text-center font-bold">Accounts</p>
            <ul className="flex flex-col gap-4">
              {accounts.map((account) => (
                <li key={account.id} className="flex flex-col gap-2">
                  <p className="text-lg text-center">{account.username}</p>
                  <p className="text-sm text-center">{account.address}</p>
                </li>
              ))}
            </ul>
          </div>
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center h-full">
          <p className="text-2xl text-center font-bold">Accounts</p>
          <p className="text-lg text-center">
            {isConnected ? "You have no accounts" : "Please connect your wallet"}
          </p>
        </div>
      )}
    </div>
  );
};
