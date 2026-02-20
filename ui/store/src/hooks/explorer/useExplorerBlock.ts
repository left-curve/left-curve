import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "../usePublicClient.js";

import type { IndexedBlock } from "@left-curve/dango/types";

export type ExplorerBlockState = {
  searchBlock: IndexedBlock | null;
  currentBlock: IndexedBlock;
  height: number;
  isFutureBlock: boolean;
  isInvalidBlock: boolean;
};

export function useExplorerBlock(block: string) {
  const client = usePublicClient();

  return useQuery<ExplorerBlockState>({
    queryKey: ["block_explorer", block],
    queryFn: async () => {
      const isLatest = block === "latest";
      const parsedHeight = Number(block);

      const [searchBlock, currentBlock] = await Promise.all([
        Number.isNaN(parsedHeight) && !isLatest
          ? null
          : client.queryBlock(isLatest ? undefined : { height: parsedHeight }),
        client.queryBlock(),
      ]);

      const isFutureBlock = parsedHeight > 0 && parsedHeight > currentBlock.blockHeight;
      const isInvalidBlock = (!isLatest && Number.isNaN(parsedHeight)) || parsedHeight < 0;

      return {
        searchBlock,
        currentBlock,
        height: parsedHeight,
        isFutureBlock,
        isInvalidBlock,
      };
    },
    enabled: !!block,
  });
}
