import { isValidAddress } from "@left-curve/dango";
import { wait } from "@left-curve/dango/utils";
import { usePublicClient } from "@left-curve/store";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import fuzzysort from "fuzzysort";
import { useState } from "react";

import { applets as AppletsMetadata } from "../../applets";

import type { AppletMetadata } from "@left-curve/applets-kit";
import type {
  Address,
  AppConfig,
  ContractInfo,
  IndexedBlock,
  IndexedTransaction,
} from "@left-curve/dango/types";

type UseSearchBarParameters = {
  debounceMs?: number;
};

export function useSearchBar(parameters: UseSearchBarParameters = {}) {
  const { debounceMs = 300 } = parameters;
  const [searchText, setSearchText] = useState("");
  const [block, setBlock] = useState<IndexedBlock>();
  const [txs, setTxs] = useState<IndexedTransaction[]>([]);
  const [applets, setApplets] = useState<AppletMetadata[]>(AppletsMetadata.slice(0, 4));
  const [contractInfo, setContractInfo] = useState();

  const queryClient = useQueryClient();
  const client = usePublicClient();

  const { data, ...query } = useQuery({
    queryKey: ["searchBar", searchText],
    queryFn: async ({ signal }) => {
      if (!searchText.length) {
        setApplets(AppletsMetadata.slice(0, 4));
        setBlock(undefined);
        setTxs([]);
        return null;
      }

      setApplets(
        fuzzysort
          .go(searchText, AppletsMetadata, {
            threshold: 0.5,
            all: false,
            keys: ["title", "description", (obj: AppletMetadata) => obj.keywords?.join()],
          })
          .map(({ obj }) => obj),
      );

      await wait(debounceMs);
      if (signal.aborted) return;

      const promises: Promise<unknown>[] = [];
      const { addresses } = await client.getAppConfig();

      const response = await client.queryWasmSmart({
        contract: addresses.accountFactory,
        msg: { codeHashes: {} },
      });

      if (isValidAddress(searchText)) {
        // search for contract
        promises.push(
          (async () => {
            const contractInfo = await client.getContractInfo({ address: searchText as Address });
          })(),
        );
      } else if (searchText.length === 64) {
        // search for tx hash
        promises.push(
          (async () => {
            const tx = await client.searchTx({ hash: searchText });
            if (tx) setTxs([tx]);
            queryClient.setQueryData(["tx", searchText], tx);
          })(),
        );
      } else if (!Number.isNaN(Number(searchText))) {
        promises.push(
          (async () => {
            const block = await client.queryBlock({ height: +searchText });
            setBlock(block);
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

  return { searchText, setSearchText, block, txs, applets, ...query };
}
