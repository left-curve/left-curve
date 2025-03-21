import { isValidAddress } from "@left-curve/dango";
import { wait } from "@left-curve/dango/utils";
import { usePublicClient } from "@left-curve/store-react";
import { useQuery } from "@tanstack/react-query";
import fuzzysort from "fuzzysort";
import { useState } from "react";

import { applets as AppletsMetadata } from "../../applets";

import type { AppletMetadata } from "@left-curve/applets-kit";
import type { IndexedBlock } from "@left-curve/dango/types";

type UseSearchBarParameters = {
  debounceMs?: number;
};

export function useSearchBar(parameters: UseSearchBarParameters = {}) {
  const { debounceMs = 100 } = parameters;
  const [searchText, setSearchText] = useState("");
  const [block, setBlock] = useState<IndexedBlock>();
  const [txs, setTxs] = useState<[]>();
  const [applets, setApplets] = useState<AppletMetadata[]>(AppletsMetadata.slice(0, 4));

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

      if (isValidAddress(searchText)) {
        // search for contract
        promises.push((async () => {})());
      } else if (!Number.isNaN(Number(searchText))) {
        promises.push(
          (async () => {
            const block = await client.queryBlock({ height: +searchText });
            setBlock(block);
          })(),
        );
      } else {
        // search for username
        promises.push((async () => {})());
        // search for tokens
      }

      return Promise.allSettled(promises);
    },
  });

  return { searchText, setSearchText, block, txs, applets, ...query };
}
