import type React from "react";
import { Button, Input, PoolCard, SearchIcon } from "../";

import type { Pool, PoolId } from "@left-curve/dango/types";

interface Props {
  onPoolSelected: (id: number) => void;
}

export const PoolSelector: React.FC<Props> = ({ onPoolSelected }) => {
  const pools = {} as Record<PoolId, Pool>;

  return (
    <div className="dango-grid-4x4-M flex flex-col items-center gap-9">
      <h2 className="font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest mb-1">
        SELECT POOL
      </h2>
      <div className="flex flex-col gap-4 w-full">
        <Input
          classNames={{
            inputWrapper:
              "bg-surface-purple-100 border border-purple-600/40 group-hover:bg-surface-purple-200 text-typography-black-100 px-2 rounded-2xl",
            input: "placeholder:text-typography-black-100/40 text-typography-black-100 text-xl",
          }}
          placeholder="Search tokens"
          startContent={<SearchIcon className="h-6 w-6 text-typography-black-100/40" />}
        />
        <div className="flex flex-col gap-1">
          <div className="px-6 gap-1 grid grid-cols-[1fr_80px_80px_80px] text-end">
            <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest text-start">
              ASSET
            </p>
            <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
              BALANCE
            </p>
            <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
              TVL
            </p>
            <p className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
              APR
            </p>
          </div>
          {Object.entries(pools).map(([id, pool]) => (
            <PoolCard
              key={id}
              poolId={Number(id)}
              pool={pool}
              onPoolSelected={(id) => onPoolSelected(id)}
            />
          ))}
        </div>
      </div>
      <Button variant="light">Show all</Button>
    </div>
  );
};
