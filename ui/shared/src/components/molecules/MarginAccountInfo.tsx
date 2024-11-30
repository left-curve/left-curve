import { useAccount } from "@left-curve/react";
import type React from "react";
import { Button } from "../atoms/Button";
import { BorrowingBar } from "./BorrowingBar";

import type { Account } from "@left-curve/types";

interface Props {
  avatarUrl: string;
  account: Account;
}

export const MarginAccountInfo: React.FC<Props> = ({ avatarUrl, account }) => {
  const { account: selectedAccount } = useAccount();

  const isCurrentAccount = selectedAccount?.address === account.address;

  return (
    <div className="dango-grid-4x8-L flex flex-col gap-3 items-center justify-center text-sand-900">
      <div className="flex gap-4 flex-col md:grid md:grid-cols-3 w-full items-center md:items-end justify-center mb-14">
        <div className="flex flex-col gap-2 order-2 md:order-1 items-center md:items-start justify-center">
          <p className="uppercase text-typography-black-400 tracking-widest font-semibold text-sm">
            TOTAL COLLATERAL
          </p>
          <div className="flex gap-1 font-bold">
            <p className="text-typography-black-200 font-extrabold text-3xl">$ 123.48</p>
          </div>
        </div>

        <div className="flex flex-col gap-4 order-1 md:order-2 items-center justify-center">
          <p className="text-typography-black-200 uppercase font-bold text-base tracking-widest">
            {account.username} {account.type} #{account.index}
          </p>
          <div className="rounded-full h-[9.5rem] w-[9.5rem] bg-surface-rose-200 p-2">
            <img src={avatarUrl} alt="margin" className="rounded-full h-full w-full" />
          </div>
        </div>

        <div className="flex flex-col gap-2 order-3 items-center md:items-end justify-center">
          <p className="uppercase text-typography-black-400 tracking-widest font-semibold text-sm">
            TOTAL DEBT
          </p>
          <div className="flex gap-1 font-bold">
            <p className="text-typography-black-200 font-extrabold text-3xl">$ 23.10 K</p>
          </div>
        </div>
      </div>

      <div className="w-full max-w-[20rem]">
        <BorrowingBar total={120000} current={85000} threshold={100000} />
      </div>

      {isCurrentAccount ? <Button variant="light">Rename</Button> : null}
    </div>
  );
};
