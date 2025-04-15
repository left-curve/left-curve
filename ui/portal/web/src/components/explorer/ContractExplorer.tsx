import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import { createContext, useContext } from "react";

import {
  Badge,
  Cell,
  Table,
  type TableColumn,
  TextCopy,
  TruncateText,
  useMediaQuery,
} from "@left-curve/applets-kit";
import type { Address, Coins, ContractInfo, Denom } from "@left-curve/dango/types";
import { camelToTitleCase } from "@left-curve/dango/utils";
import { useConfig, usePrices, usePublicClient } from "@left-curve/store";
import type { AnyCoin, WithAmount, WithPrice } from "@left-curve/store/types";
import type React from "react";
import type { PropsWithChildren } from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { ContractCard } from "../foundation/ContractCard";
import { HeaderExplorer } from "./HeaderExplorer";

const ContractExplorerContext = createContext<
  | (UseQueryResult<(ContractInfo & { name: string; balances: Coins }) | null, Error> & {
      address: string;
    })
  | null
>(null);

const useContractExplorer = () => {
  const context = useContext(ContractExplorerContext);
  if (context === null) {
    throw new Error("useContractExplorer must be used within a ContractExplorerProvider");
  }
  return context;
};

type ContractExplorerProps = {
  address: Address;
};

const Root: React.FC<PropsWithChildren<ContractExplorerProps>> = ({ address, children }) => {
  const client = usePublicClient();
  const { getAppConfig } = useConfig();

  const query = useQuery({
    queryKey: ["contract_explorer", address],
    queryFn: async () => {
      const [appConfig, contractInfo, balances] = await Promise.all([
        getAppConfig(),
        client.getContractInfo({ address }),
        client.getBalances({ address }),
      ]);

      const isAccount = Object.values(appConfig.accountFactory.codeHashes).includes(
        contractInfo.codeHash,
      );

      if (isAccount) return null;

      const appContract = Object.entries(appConfig.addresses).find(
        ([_, cAddress]) => cAddress === address,
      );
      const name = appContract
        ? `Dango ${camelToTitleCase(appContract[0])}`
        : (contractInfo.label ?? "Contract");

      return {
        ...contractInfo,
        name,
        address,
        balances,
      };
    },
  });

  return (
    <ContractExplorerContext.Provider value={{ address, ...query }}>
      <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">{children}</div>
    </ContractExplorerContext.Provider>
  );
};

const Details: React.FC = () => {
  const { isLoading, data: contract, address } = useContractExplorer();
  const { calculateBalance } = usePrices();
  const { isMd } = useMediaQuery();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  if (!contract || isLoading) return null;

  const { name, codeHash, admin, balances } = contract;
  const totalCoins = Object.values(balances).length;
  const totalBalance = calculateBalance(balances, {
    format: true,
    formatOptions: { ...formatNumberOptions, currency: "usd" },
  });

  return (
    <div className="flex flex-col gap-6 lg:flex-row">
      <ContractCard name={name} address={address} balance={totalBalance} />
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
  const { isLoading, data: contract, address } = useContractExplorer();
  if (isLoading || contract) return null;

  return (
    <div className="w-full md:max-w-[76rem] p-4">
      <HeaderExplorer>
        <div className="flex flex-col gap-2 items-center border border-red-bean-50">
          <h3 className="exposure-m-italic text-gray-700">
            {m["explorer.contracts.notFound.title"]()}
          </h3>
          <p className="diatype-m-medium max-w-[42.5rem] text-center text-gray-500 ">
            {m["explorer.contracts.notFound.pre"]()}
            <span className="break-all overflow-hidden underline"> {address}</span>{" "}
            {m["explorer.contracts.notFound.description"]()}
          </p>
        </div>
      </HeaderExplorer>
    </div>
  );
};

const Assets: React.FC = () => {
  const { isLoading, data: contract } = useContractExplorer();
  const { coins: allCoins, state } = useConfig();
  const { settings } = useApp();
  const { getPrice } = usePrices();
  const { formatNumberOptions } = settings;

  if (isLoading || !contract) return null;

  const { balances } = contract;
  const coins = allCoins[state.chainId];

  const data = Object.entries(balances).map(([denom, amount]) => {
    const coin = coins[denom];
    const price = getPrice(amount, denom, { format: true, formatOptions: formatNumberOptions });

    return { ...coin, price, amount };
  });

  const columns: TableColumn<WithAmount<WithPrice<AnyCoin>>> = [
    {
      header: "Asset",
      cell: ({ row }) => <Cell.Asset asset={row.original} />,
    },
    {
      header: "Market Price",
      cell: ({ row }) => (
        <Cell.MarketPrice denom={row.original.denom} formatOptions={formatNumberOptions} />
      ),
    },
    {
      header: "Available",
      cell: ({ row }) => (
        <Cell.Amount
          amount={row.original.amount}
          price={row.original.price}
          decimals={row.original.decimals}
        />
      ),
    },
    {
      header: "Total",
      cell: ({ row }) => (
        <Cell.Amount
          className="text-end"
          amount={row.original.amount}
          price={row.original.price}
          decimals={row.original.decimals}
        />
      ),
    },
  ];

  return <Table data={data} columns={columns} />;
};

export const ContractExplorer = Object.assign(Root, {
  Details,
  NotFound,
  Assets,
});
