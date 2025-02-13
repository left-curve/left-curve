import { Link, createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

import {
  AccountCard,
  Button,
  IconAddCross,
  StrategyCard,
  Table,
  twMerge,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { motion } from "framer-motion";

export const Route = createFileRoute("/(app)/_app/")({
  component: OverviewComponent,
});

function OverviewComponent() {
  const { account } = useAccount();
  const [tableActive, setTableActive] = useState<"Assets" | "Earn" | "Pools">("Assets");
  return (
    <div className="w-full  md:max-w-[76rem] mx-auto flex flex-col gap-8 p-4">
      <div className="rounded-3xl bg-rice-50 shadow-card-shadow flex flex-col md:flex-row gap-4 w-full p-4 items-center md:items-start">
        <AccountCard account={account!} balance="125.04M" balanceChange="0.05%" />
        <div className="w-full flex flex-col gap-4 items-center">
          {/*  assets component */}
          <div className="hidden md:flex flex-col bg-rice-25 [box-shadow:0px_-1px_2px_0px_#F1DBBA80,_0px_2px_4px_0px_#AB9E8A66] rounded-md p-4 gap-4 w-full">
            <div className="flex items-center justify-between w-full">
              <p className="text-md font-bold">Assets</p>
              <Button as={Link} variant="link" size="xs">
                View all
              </Button>
            </div>
            <div className="flex flex-wrap gap-4 items-center justify-between">
              {/* Assets item component */}
              {Array.from([1, 2, 3, 4, 5]).map((e, i) => {
                return (
                  <div className="flex gap-2 items-center" key={`asset-${e}`}>
                    <img
                      src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                      alt=""
                      className="rounded-xl h-7 w-7"
                    />
                    <div className="flex flex-col text-xs">
                      <p>Ethereum</p>
                      <p className="text-gray-500">$124.05</p>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="md:self-end flex gap-4 items-center justify-center w-full md:max-w-[256px]">
            <Button fullWidth size="md">
              Fund
            </Button>
            <Button fullWidth variant="secondary" size="md">
              Send
            </Button>
          </div>
        </div>
      </div>
      {/* second component */}
      <div className="flex gap-4 md:gap-8 items-start flex-wrap md:justify-start w-full">
        {/* applets items */}
        <div className="flex flex-col items-center gap-2">
          <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-green-bean-50 rounded-md p-[10px]">
            <img src="/images/applets/swap.svg" alt="" className="w-full h-full" />
          </div>
          <p className="text-sm font-bold">Swap</p>
        </div>
        <div className="flex flex-col items-center gap-2">
          <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-red-bean-50 rounded-md p-[10px]">
            <img src="/images/applets/earn.svg" alt="" className="w-full h-full" />
          </div>
          <p className="text-sm font-bold">Earn</p>
        </div>
        <div className="flex flex-col items-center gap-2">
          <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-rice-50 rounded-md p-[10px]">
            <img src="/images/applets/multisig.svg" alt="" className="w-full h-full" />
          </div>
          <p className="text-sm font-bold">Multisig</p>
        </div>
        {/* add applets item */}
        <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow border-[1.43px] border-rice-100 text-rice-100 rounded-md p-[10px] flex items-center justify-center">
          <IconAddCross />
        </div>
      </div>

      <div className="bg-rice-25 shadow-card-shadow flex flex-col rounded-3xl w-full">
        <p className="h3-heavy font-extrabold px-4 py-3">Top Yields</p>

        <div className="flex gap-6 w-full overflow-y-scroll p-4 scrollbar-none">
          {Array.from([1, 2, 3]).map(() => (
            <StrategyCard key={crypto.randomUUID()} />
          ))}
        </div>
      </div>

      <Table
        topContent={
          <motion.ul className="flex text-base relative  items-center w-fit bg-green-bean-200 p-1 rounded-md">
            {Array.from(["Assets", "Earn", "Pools"]).map((e, i) => {
              const isActive = e === tableActive;
              return (
                <motion.li
                  className="relative transition-all flex items-center justify-center py-2 px-4 cursor-pointer"
                  key={`navLink-${e}`}
                  onClick={() => setTableActive(e as any)}
                >
                  <p
                    className={twMerge(
                      "italic font-medium font-exposure transition-all relative z-10",
                      isActive ? "text-black" : "text-gray-300",
                    )}
                  >
                    {e}
                  </p>
                  {isActive ? (
                    <motion.div
                      className="w-full h-full rounded-[10px] bg-green-bean-50 absolute bottom-0 left-0 [box-shadow:0px_4px_6px_2px_#1919191F]"
                      layoutId="active"
                    />
                  ) : null}
                </motion.li>
              );
            })}
          </motion.ul>
        }
        bottomContent={
          <Button variant="secondary" className="self-center">
            View All
          </Button>
        }
      />
    </div>
  );
}
