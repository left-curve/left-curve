import { AddressVisualizer } from "@left-curve/applets-kit";
import { useInfiniteGraphqlQuery, usePrices, usePublicClient } from "@left-curve/store";
import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { createContext, useContext } from "react";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";

import { Badge, TextCopy } from "@left-curve/applets-kit";
import { AccountCard } from "../foundation/AccountCard";
import { AssetsTable } from "./AssetsTable";
import { HeaderExplorer } from "./HeaderExplorer";

import type {
  Account,
  Address,
  Coins,
  ContractInfo,
  IndexedTransaction,
} from "@left-curve/dango/types";
import type React from "react";
import type { PropsWithChildren } from "react";
import { TransactionsTable } from "./TransactionsTable";

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
  const navigate = useNavigate();
  const { calculateBalance } = usePrices();
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
    <div className="flex flex-col gap-4 lg:flex-row">
      <AccountCard account={account} balance={totalBalance} />
      <div className="flex flex-col gap-4 rounded-xl p-4 bg-bg-secondary-rice shadow-account-card relative overflow-hidden w-full min-h-[10rem]">
        <h4 className="h4-bold">{m["explorer.contracts.details.contractDetails"]()}</h4>
        <div className="flex flex-col gap-2">
          <div className="flex md:items-center gap-1 flex-col md:flex-row">
            <p className="diatype-sm-medium text-tertiary-500 md:min-w-[8rem]">
              {m["explorer.contracts.details.codeHash"]()}
            </p>

            <p className="diatype-sm-medium break-all whitespace-normal">
              {codeHash}
              <TextCopy className="w-4 h-4 text-tertiary-500 ml-1" copyText={codeHash} />
            </p>
          </div>
          <div className="flex md:items-center gap-1 flex-col md:flex-row">
            <p className="diatype-sm-medium text-tertiary-500 md:min-w-[8rem]">
              {m["explorer.contracts.details.admin"]()}
            </p>
            {admin ? (
              <AddressVisualizer
                classNames={{ text: "diatype-sm-medium" }}
                address={admin}
                withIcon
                onClick={(url) => navigate({ to: url })}
              />
            ) : (
              <p className="diatype-sm-medium">None</p>
            )}
          </div>
          <div className="flex md:items-center gap-1 flex-col md:flex-row">
            <p className="diatype-sm-medium text-tertiary-500 md:min-w-[8rem]">
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
          <p className="diatype-m-medium max-w-[42.5rem] text-center text-tertiary-500 ">
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

const Transactions: React.FC = () => {
  const { isLoading, data: account } = useAccountExplorer();
  const client = usePublicClient();

  const { data, pagination, ...transactions } = useInfiniteGraphqlQuery<IndexedTransaction>({
    limit: 10,
    sortBy: "BLOCK_HEIGHT_DESC",
    query: {
      enabled: !!account,
      queryKey: ["account_transactions", account?.address],
      queryFn: async ({ pageParam }) =>
        client.searchTxs({ senderAddress: account?.address, ...pageParam }),
    },
  });

  if (isLoading || !account) return null;

  return (
    <TransactionsTable
      transactions={data?.pages[pagination?.currentPage - 1]?.nodes || []}
      pagination={{ ...pagination, isLoading: transactions.isLoading }}
    />
  );
};

export const AccountExplorer = Object.assign(Root, {
  Details,
  NotFound,
  Assets,
  Transactions,
});
