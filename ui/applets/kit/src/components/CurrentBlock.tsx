import { useApp } from "@left-curve/foundation";
import { useEffect, useState } from "react";
import { Skeleton } from "./Skeleton";

import type React from "react";
import type { BlockInfo } from "@left-curve/dango/types";

type CurrentBlockProps = {
  classNames?: {
    skeleton?: string;
    container?: string;
  };
  selector?: "timestamp" | "height" | "hash";
};

export const CurrentBlock: React.FC<CurrentBlockProps> = ({ classNames, selector = "height" }) => {
  const [currentBlock, setCurrentBlock] = useState<BlockInfo>();
  const { subscriptions } = useApp();

  useEffect(() => {
    const unsubscribe = subscriptions.subscribe("block", {
      listener: ({ blockHeight, hash, createdAt }) => {
        setCurrentBlock({
          height: blockHeight.toString(),
          hash,
          timestamp: new Date(createdAt).toJSON(),
        });
      },
    });
    return () => unsubscribe();
  }, []);

  return currentBlock ? (
    <p
      className={classNames?.container}
    >{`${selector === "height" ? "#" : ""}${currentBlock[selector]}`}</p>
  ) : (
    <Skeleton className={classNames?.skeleton} />
  );
};
