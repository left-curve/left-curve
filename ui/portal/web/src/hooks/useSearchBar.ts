import { isValidAddress } from "@left-curve/dango";
import { camelToTitleCase, wait } from "@left-curve/dango/utils";
import { useConfig, usePublicClient } from "@left-curve/store";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import fuzzysort from "fuzzysort";
import { useMemo, useReducer, useState } from "react";
import { useFavApplets } from "./useFavApplets";

import { APPLETS } from "~/constants";

import type { AppletMetadata } from "@left-curve/applets-kit";
import type {
  Account,
  Address,
  ContractInfo,
  IndexedBlock,
  IndexedTransaction,
} from "@left-curve/dango/types";

type UseSearchBarParameters = {
  debounceMs?: number;
};

export type SearchBarResult = {
  block?: IndexedBlock;
  txs: IndexedTransaction[];
  applets: AppletMetadata[];
  contract?: ContractInfo & { name: string; address: Address };
  account?: Account;
};

export function useSearchBar(parameters: UseSearchBarParameters = {}) {
  const { debounceMs = 300 } = parameters;
  const [searchText, setSearchText] = useState("");
  const { favApplets } = useFavApplets();

  const noResult: SearchBarResult = useMemo(
    () => ({
      block: undefined,
      txs: [],
      applets: Object.values(favApplets).slice(0, 5),
      contract: undefined,
      account: undefined,
    }),
    [favApplets],
  );

  const [searchResult, setSearchResult] = useReducer(
    (os: SearchBarResult, ns: Partial<SearchBarResult>) => ({ ...os, ...ns }),
    noResult,
  );

  const { getAppConfig } = useConfig();
  const queryClient = useQueryClient();
  const client = usePublicClient();

  const { data, ...query } = useQuery({
    queryKey: ["searchBar", searchText],
    queryFn: async ({ signal }) => {
      if (!searchText.length) {
        setSearchResult(noResult);
        return null;
      }

      setSearchResult({
        applets: fuzzysort
          .go(searchText, APPLETS, {
            threshold: 0.5,
            all: false,
            keys: ["title", "description", (obj: AppletMetadata) => obj.keywords?.join()],
          })
          .map(({ obj }) => obj),
      });

      await wait(debounceMs);
      if (signal.aborted) return;

      const promises: Promise<unknown>[] = [];
      const { accountFactory, addresses } = await getAppConfig();

      if (isValidAddress(searchText)) {
        // search for contract
        promises.push(
          (async () => {
            const contractInfo = await client.getContractInfo({ address: searchText as Address });
            const isAccount = Object.values(accountFactory.codeHashes).includes(
              contractInfo.codeHash,
            );

            if (isAccount) {
              const account = await client.getAccountInfo({ address: searchText as Address });
              setSearchResult({ account: account ? account : undefined });
            } else {
              const appContract = Object.entries(addresses).find(
                ([_, address]) => address === searchText,
              );
              const name = appContract
                ? `Dango ${camelToTitleCase(appContract[0])}`
                : (contractInfo.label ?? "Contract");
              setSearchResult({
                contract: { ...contractInfo, name, address: searchText as Address },
              });
            }
          })(),
        );
      } else if (searchText.length === 64) {
        // search for tx hash
        promises.push(
          (async () => {
            const tx = await client.searchTx({ hash: searchText });
            if (tx) setSearchResult({ txs: [tx] });
            queryClient.setQueryData(["tx", searchText], tx);
          })(),
        );
      } else if (!Number.isNaN(Number(searchText))) {
        promises.push(
          (async () => {
            const block = await client.queryBlock({ height: +searchText });
            setSearchResult({ block });
            queryClient.setQueryData(["block", searchText], block);
          })(),
        );
      } else {
        // search for username
        promises.push((async () => {})());
        // search for tokens
      }

      return await Promise.allSettled(promises);
    },
  });

  return { searchText, setSearchText, searchResult, ...query };
}
