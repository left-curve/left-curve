import { isValidAddress } from "@left-curve/dango";
import { wait } from "@left-curve/dango/utils";
import { useConfig, usePublicClient } from "@left-curve/store";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import fuzzysort from "fuzzysort";
import { useMemo, useReducer, useState } from "react";

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
  applets: Record<string, AppletMetadata>;
  favApplets: string[];
};

export type SearchBarResult = {
  block?: IndexedBlock;
  txs: IndexedTransaction[];
  applets: AppletMetadata[];
  contract?: ContractInfo & { address: Address };
  account?: Account;
};

export function useSearchBar(parameters: UseSearchBarParameters) {
  const applets = Object.values(parameters.applets);
  const { debounceMs = 300, favApplets } = parameters;
  const [searchText, setSearchText] = useState("");

  const noResult: SearchBarResult = useMemo(
    () => ({
      block: undefined,
      txs: [],
      applets: Object.values(applets.filter((applet) => favApplets.includes(applet.id))),
      contract: undefined,
      account: undefined,
    }),
    [applets, favApplets],
  );

  const [searchResult, setSearchResult] = useReducer(
    (os: SearchBarResult, ns: Partial<SearchBarResult>) => ({ ...os, ...ns }),
    noResult,
  );

  const { getAppConfig } = useConfig();
  const queryClient = useQueryClient();
  const client = usePublicClient();

  const allNotFavApplets = useMemo(() => {
    return Object.values(applets).filter((applet) => !favApplets.includes(applet.id));
  }, [applets, favApplets]);

  const { data: _, ...query } = useQuery({
    queryKey: ["searchBar", searchText, favApplets],
    queryFn: async ({ signal }) => {
      if (!searchText.length) {
        setSearchResult(noResult);
        return null;
      }

      setSearchResult({
        applets: fuzzysort
          .go(searchText, applets, {
            threshold: 0.5,
            all: false,
            keys: ["title", "description", (obj: AppletMetadata) => obj.keywords?.join()],
          })
          .map(({ obj }) => obj),
      });

      await wait(debounceMs);
      if (signal.aborted) return;

      const promises: Promise<unknown>[] = [];
      const { accountFactory } = await getAppConfig();

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
              setSearchResult({
                contract: { ...contractInfo, address: searchText as Address },
              });
            }
          })(),
        );
      } else if (searchText.length === 64) {
        // search for tx hash
        promises.push(
          (async () => {
            const txs = await client.searchTxs({ hash: searchText });
            if (txs.nodes.length) {
              setSearchResult({ txs: txs.nodes });
              queryClient.setQueryData(["tx", searchText], txs.nodes[0]);
            }
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

  return { searchText, setSearchText, searchResult, allNotFavApplets, ...query };
}
