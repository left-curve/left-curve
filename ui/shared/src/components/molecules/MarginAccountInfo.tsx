import { useAccount } from "@leftcurve/react";
import type React from "react";
import { Button } from "../atoms/Button";
import { BorrowingBar } from "./BorrowingBar";

interface Props {
  avatarUrl: string;
}

export const MarginAccountInfo: React.FC<Props> = ({ avatarUrl }) => {
  const { account } = useAccount();
  if (!account) return null;

  return (
    <div className="bg-gradient-to-br from-sand-100/70 to-white/10 backdrop-blur-sm  rounded-3xl flex flex-col gap-3 items-center justify-center text-sand-900 p-4 md:min-w-[18rem] md:w-fit w-full  min-h-[18rem]  md:max-w-2xl">
      <div className="flex gap-4 flex-col md:grid md:grid-cols-3 w-full items-center md:items-end justify-center md:gap-20 mb-14">
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

      <Button color="sand" variant="light" className="italic">
        Edit
      </Button>
    </div>
  );
};
