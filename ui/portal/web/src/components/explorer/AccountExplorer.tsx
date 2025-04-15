import { useMediaQuery } from "@left-curve/applets-kit";
import { usePrices, usePublicClient } from "@left-curve/store";
import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import { createContext, useContext } from "react";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";

import { Badge, TextCopy, TruncateText } from "@left-curve/applets-kit";
import { AccountCard } from "../foundation/AccountCard";
import { AssetsTable } from "./AssetsTable";
import { HeaderExplorer } from "./HeaderExplorer";

import type { Account, Address, Coins, ContractInfo } from "@left-curve/dango/types";
import type React from "react";
import type { PropsWithChildren } from "react";

const AccountExplorerContext = createContext<
  | (UseQueryResult<(Account & ContractInfo & { balances: Coins }) | null, Error> & {
      address: string;
    })
  | null
>(null);

const useAccountExplorer = () => {
  const context = useContext(AccountExplorerContext);
  if (context === null) {
    throw new Error("useAccountExplorer must be used within a AccountExplorerContext");
  }
  return context;
};

type AccountExplorerProps = {
  address: Address;
};

const Root: React.FC<PropsWithChildren<AccountExplorerProps>> = ({ address, children }) => {
  const client = usePublicClient();

  const query = useQuery({
    queryKey: ["account_explorer", address],
    queryFn: async () => {
      const [account, contractInfo, balances] = await Promise.all([
        client.getAccountInfo({ address }),
        client.getContractInfo({ address }),
        client.getBalances({ address }),
      ]);

      if (!account) return null;

      return {
        ...account,
        ...contractInfo,
        balances,
      };
    },
  });

  return (
    <AccountExplorerContext.Provider value={{ address, ...query }}>
      <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">{children}</div>
    </AccountExplorerContext.Provider>
  );
};

const Details: React.FC = () => {
  const { isLoading, data: account } = useAccountExplorer();
  const { calculateBalance } = usePrices();
  const { isMd } = useMediaQuery();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  if (!account || isLoading) return null;

  const { codeHash, admin, balances } = account;
  const totalCoins = Object.values(balances).length;
  const totalBalance = calculateBalance(balances, {
    format: true,
    formatOptions: { ...formatNumberOptions, currency: "usd" },
  });

  return (
    <div className="flex flex-col gap-6 lg:flex-row">
      <AccountCard account={account} balance={totalBalance} />
      <div className="flex flex-col gap-4 rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow relative overflow-hidden w-full">
        <h4 className="h4-heavy">{m["explorer.contracts.details.contractDetails"]()}</h4>
        <div className="flex flex-col gap-2">
          <div className="flex gap-1 items-center">
            <p className="diatype-md-medium text-gray-500">
              {m["explorer.contracts.details.codeHash"]()}
            </p>
            {isMd ? (
              <p className="diatype-m-bold">{codeHash}</p>
            ) : (
              <TruncateText text={codeHash} className="diatype-m-bold" />
            )}
            <TextCopy className="w-4 h-4 text-gray-500" copyText={""} />
          </div>
          <div className="flex gap-1 items-center">
            <p className="diatype-md-medium text-gray-500">
              {m["explorer.contracts.details.admin"]()}
            </p>
            <p className="diatype-m-bold">{admin ? admin : "None"}</p>
          </div>
          <div className="flex gap-1 items-center">
            <p className="diatype-md-medium text-gray-500">
              {m["explorer.contracts.details.balances"]()}
            </p>
            <Badge color="green" size="m" text={`${totalBalance} (${totalCoins} Assets)`} />
          </div>
        </div>
      </div>
    </div>
  );
};

const NotFound: React.FC = () => {
  const { isLoading, data: account, address } = useAccountExplorer();
  if (isLoading || account) return null;

  return (
    <div className="w-full md:max-w-[76rem] p-4">
      <HeaderExplorer>
        <div className="flex flex-col gap-2 items-center border border-red-bean-50">
          <h3 className="exposure-m-italic text-gray-700">
            {m["explorer.accounts.notFound.title"]()}
          </h3>
          <p className="diatype-m-medium max-w-[42.5rem] text-center text-gray-500 ">
            {m["explorer.accounts.notFound.pre"]()}
            <span className="break-all overflow-hidden underline">{address}</span>{" "}
            {m["explorer.accounts.notFound.description"]()}
          </p>
        </div>
      </HeaderExplorer>
    </div>
  );
};

const Assets: React.FC = () => {
  const { isLoading, data: account } = useAccountExplorer();

  if (isLoading || !account) return null;

  return <AssetsTable balances={account.balances} />;
};

export const AccountExplorer = Object.assign(Root, {
  Details,
  NotFound,
  Assets,
});
